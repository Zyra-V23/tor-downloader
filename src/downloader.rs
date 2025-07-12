use crate::config::Config;
use crate::errors::{download_error, tor_connection_error, Result};
use bytes::Bytes;
use log::{debug, info, warn, error};
use reqwest::{Client, Proxy};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};

#[derive(Clone)]
pub struct Downloader {
    client: Client,
    config: Arc<Config>,
    stats: Arc<tokio::sync::Mutex<DownloadStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct DownloadStats {
    pub total_requests: usize,
    pub successful_downloads: usize,
    pub failed_downloads: usize,
    pub bytes_downloaded: u64,
    pub retry_count: usize,
}

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub url: String,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub success: bool,
    pub error: Option<String>,
}

impl Downloader {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let proxy_url = format!("socks5h://{}", config.proxy);
        info!("🔧 Configuring SOCKS5H proxy: {}", proxy_url);
        
        let proxy = Proxy::all(&proxy_url)
            .map_err(|e| tor_connection_error(&format!("Failed to create proxy: {}", e)))?;

        debug!("✅ SOCKS5H proxy configured successfully");

        let client = Client::builder()
            .proxy(proxy)
            .user_agent(&config.user_agent)
            .timeout(Duration::from_secs(config.timeout_sec))
            .danger_accept_invalid_certs(true) // Common for .onion sites
            .no_proxy() // Ensure no system proxy bypasses our SOCKS5
            .build()
            .map_err(|e| tor_connection_error(&format!("Failed to create HTTP client: {}", e)))?;
        
        info!("🚀 HTTP client created successfully with SOCKS5H proxy");

        Ok(Downloader {
            client,
            config,
            stats: Arc::new(tokio::sync::Mutex::new(DownloadStats::default())),
        })
    }

    pub async fn download_url(&self, url: &str) -> Result<DownloadResult> {
        let mut attempts = 0;
        let max_attempts = self.config.max_retries + 1;

        loop {
            attempts += 1;
            
            // Update stats
            {
                let mut stats = self.stats.lock().await;
                stats.total_requests += 1;
                if attempts > 1 {
                    stats.retry_count += 1;
                }
            }

            debug!("Downloading {} (attempt {}/{})", url, attempts, max_attempts);

            match self.try_download(url).await {
                Ok(result) => {
                    if result.success {
                        let mut stats = self.stats.lock().await;
                        stats.successful_downloads += 1;
                        if let Some(size) = result.content_length {
                            stats.bytes_downloaded += size;
                        }
                    }
                    return Ok(result);
                }
                Err(e) => {
                    warn!("Download attempt {} failed for {}: {}", attempts, url, e);
                    
                    if attempts >= max_attempts {
                        let mut stats = self.stats.lock().await;
                        stats.failed_downloads += 1;
                        
                        return Ok(DownloadResult {
                            url: url.to_string(),
                            content_type: None,
                            content_length: None,
                            success: false,
                            error: Some(format!("Max retries exceeded: {}", e)),
                        });
                    }

                    // Exponential backoff with jitter
                    let delay = Duration::from_millis(
                        self.config.delay_ms * (2_u64.pow(attempts as u32 - 1)) + 
                        (rand::random::<u64>() % 1000)
                    );
                    
                    debug!("Retrying in {:?}", delay);
                    sleep(delay).await;
                }
            }
        }
    }

    async fn try_download(&self, url: &str) -> Result<DownloadResult> {
        // Apply delay between requests
        if self.config.delay_ms > 0 {
            sleep(Duration::from_millis(self.config.delay_ms)).await;
        }

        // Check if we should download this URL
        if !self.config.should_download_url(url) {
            return Ok(DownloadResult {
                url: url.to_string(),
                content_type: None,
                content_length: None,
                success: false,
                error: Some("URL filtered out by include/exclude patterns".to_string()),
            });
        }

        // Make request with timeout
        debug!("📡 Sending GET request to: {}", url);
        let response = timeout(
            Duration::from_secs(self.config.timeout_sec),
            self.client.get(url).send()
        )
        .await
        .map_err(|_| {
            error!("⏰ Request timed out after {}s: {}", self.config.timeout_sec, url);
            download_error("Request timed out")
        })?
        .map_err(|e| {
            error!("🚨 HTTP request failed: {} - {}", url, e);
            download_error(&format!("HTTP request failed: {}", e))
        })?;

        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let content_length = response.content_length();

        debug!("Response: {} {} ({})", status, url, content_type.as_deref().unwrap_or("unknown"));

        // Check if response is successful
        if !response.status().is_success() {
            return Ok(DownloadResult {
                url: url.to_string(),
                content_type,
                content_length,
                success: false,
                error: Some(format!("HTTP error: {}", response.status())),
            });
        }

        // Skip download if dry run
        if self.config.dry_run {
            return Ok(DownloadResult {
                url: url.to_string(),
                content_type,
                content_length,
                success: true,
                error: None,
            });
        }

        // Get response body
        let body = response
            .bytes()
            .await
            .map_err(|e| download_error(&format!("Failed to read response body: {}", e)))?;

        // Determine output file path
        let file_path = self.config.get_output_path(url)?;
        
        // Create directories if needed
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // Write file
        fs::write(&file_path, &body)?;

        info!("Downloaded {} -> {} ({} bytes)", url, file_path.display(), body.len());

        Ok(DownloadResult {
            url: url.to_string(),
            content_type,
            content_length: Some(body.len() as u64),
            success: true,
            error: None,
        })
    }

    pub async fn download_with_content(&self, url: &str) -> Result<(DownloadResult, Option<Bytes>)> {
        let mut attempts = 0;
        let max_attempts = self.config.max_retries + 1;

        loop {
            attempts += 1;
            
            debug!("Downloading content from {} (attempt {}/{})", url, attempts, max_attempts);

            match self.try_download_content(url).await {
                Ok((result, content)) => {
                    if result.success {
                        let mut stats = self.stats.lock().await;
                        stats.successful_downloads += 1;
                        if let Some(size) = result.content_length {
                            stats.bytes_downloaded += size;
                        }
                    }
                    return Ok((result, content));
                }
                Err(e) => {
                    warn!("Download attempt {} failed for {}: {}", attempts, url, e);
                    
                    if attempts >= max_attempts {
                        let mut stats = self.stats.lock().await;
                        stats.failed_downloads += 1;
                        
                        return Ok((DownloadResult {
                            url: url.to_string(),
                            content_type: None,
                            content_length: None,
                            success: false,
                            error: Some(format!("Max retries exceeded: {}", e)),
                        }, None));
                    }

                    // Exponential backoff with jitter
                    let delay = Duration::from_millis(
                        self.config.delay_ms * (2_u64.pow(attempts as u32 - 1)) + 
                        (rand::random::<u64>() % 1000)
                    );
                    
                    debug!("Retrying in {:?}", delay);
                    sleep(delay).await;
                }
            }
        }
    }

    async fn try_download_content(&self, url: &str) -> Result<(DownloadResult, Option<Bytes>)> {
        // Apply delay between requests
        if self.config.delay_ms > 0 {
            sleep(Duration::from_millis(self.config.delay_ms)).await;
        }

        // Make request with timeout
        let response = timeout(
            Duration::from_secs(self.config.timeout_sec),
            self.client.get(url).send()
        )
        .await
        .map_err(|_| download_error("Request timed out"))?
        .map_err(|e| download_error(&format!("HTTP request failed: {}", e)))?;

        let _status = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let content_length = response.content_length();

        // Check if response is successful
        if !response.status().is_success() {
            return Ok((DownloadResult {
                url: url.to_string(),
                content_type,
                content_length,
                success: false,
                error: Some(format!("HTTP error: {}", response.status())),
            }, None));
        }

        // Get response body
        let body = response
            .bytes()
            .await
            .map_err(|e| download_error(&format!("Failed to read response body: {}", e)))?;

        Ok((DownloadResult {
            url: url.to_string(),
            content_type,
            content_length: Some(body.len() as u64),
            success: true,
            error: None,
        }, Some(body)))
    }

    pub async fn get_stats(&self) -> DownloadStats {
        self.stats.lock().await.clone()
    }

    pub async fn test_connection(&self) -> Result<()> {
        let test_url = &self.config.url;
        
        info!("🔍 Testing connection to {}", test_url);
        debug!("🌐 Using proxy: {}", self.config.proxy);

        // First, test a simple .onion site to verify Tor connectivity
        let simple_test_url = "http://duckduckgogg42ts72.onion"; // DuckDuckGo onion (updated)
        debug!("🧪 Testing Tor connectivity with .onion site: {}", simple_test_url);
        
        match timeout(
            Duration::from_secs(20),
            self.client.head(simple_test_url).send()
        ).await {
            Ok(Ok(response)) => {
                debug!("✅ Simple Tor test successful: {}", response.status());
                info!("🟢 Tor proxy is working correctly");
            }
            Ok(Err(e)) => {
                warn!("⚠️ Simple Tor test failed: {}", e);
                info!("🔄 Continuing with target URL test...");
            }
            Err(_) => {
                warn!("⚠️ Simple Tor test timed out");
                info!("🔄 Continuing with target URL test...");
            }
        }

        // Now test the actual target URL
        debug!("🎯 Testing target URL: {}", test_url);
        
        let response = timeout(
            Duration::from_secs(20),
            self.client.head(test_url).send()
        )
        .await
        .map_err(|_| {
            error!("❌ Target URL test timed out");
            tor_connection_error("Connection test timed out")
        })?
        .map_err(|e| {
            error!("❌ Target URL test failed: {}", e);
            tor_connection_error(&format!("Connection test failed: {}", e))
        })?;

        let status = response.status().as_u16();
        debug!("📊 Response status: {}", status);

        if response.status().is_success() || status == 403 || status == 404 {
            info!("✅ Connection test successful (status: {})", status);
            Ok(())
        } else {
            error!("❌ Connection test failed with status: {}", status);
            Err(tor_connection_error(&format!("Connection test failed with status: {}", status)))
        }
    }
}

impl DownloadStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_downloads as f64 / self.total_requests as f64) * 100.0
        }
    }

    pub fn average_retries(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.retry_count as f64 / self.total_requests as f64
        }
    }
} 