use crate::config::Config;
use crate::downloader::{Downloader, DownloadResult};
use crate::errors::{connection_error, Result};
use crate::monitor::Monitor;
use crate::parser::{HtmlParser, ParsedUrl, UrlType};
// use futures::stream::{self, StreamExt};
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinHandle;
use url::Url;

#[derive(Debug, Clone)]
pub struct CrawlState {
    pub visited_urls: HashSet<String>,
    pub pending_urls: VecDeque<ParsedUrl>,
    pub failed_urls: HashSet<String>,
    pub download_results: Vec<DownloadResult>,
    pub total_found: usize,
    pub total_processed: usize,
}

impl Default for CrawlState {
    fn default() -> Self {
        CrawlState {
            visited_urls: HashSet::new(),
            pending_urls: VecDeque::new(),
            failed_urls: HashSet::new(),
            download_results: Vec::new(),
            total_found: 0,
            total_processed: 0,
        }
    }
}

pub struct Crawler {
    config: Arc<Config>,
    downloader: Arc<Downloader>,
    parser: Arc<HtmlParser>,
    monitor: Arc<Monitor>,
    pub state: Arc<RwLock<CrawlState>>,
    semaphore: Arc<Semaphore>,
}

impl Crawler {
    pub fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        
        // Create output directory
        config.create_output_directory()?;
        
        // Initialize components
        let downloader = Arc::new(Downloader::new(config.clone())?);
        let parser = Arc::new(HtmlParser::new(&config.url)?);
        let monitor = Arc::new(Monitor::new(config.clone()));
        let state = Arc::new(RwLock::new(CrawlState::default()));
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));

        Ok(Crawler {
            config,
            downloader,
            parser,
            monitor,
            state,
            semaphore,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let start_time = Instant::now();
        
        info!("🚀 Starting crawl of {}", self.config.url);
        
        // Test connection first
        if !self.config.dry_run {
            self.downloader.test_connection().await?;
        }
        
        // Start monitoring if enabled
        let monitor_handle = if self.config.monitor {
            Some(self.start_monitor().await?)
        } else {
            None
        };

        // Add initial URL to pending queue
        {
            let mut state = self.state.write().await;
            state.pending_urls.push_back(ParsedUrl {
                url: self.config.url.clone(),
                url_type: UrlType::Page,
                depth: 0,
            });
            state.total_found = 1;
        }

        // Main crawl loop
        let mut active_tasks: Vec<JoinHandle<()>> = Vec::new();
        let mut consecutive_empty_rounds = 0;
        const MAX_EMPTY_ROUNDS: usize = 5;

        while consecutive_empty_rounds < MAX_EMPTY_ROUNDS {
            // Get batch of URLs to process
            let batch = self.get_next_batch().await;
            
            if batch.is_empty() {
                consecutive_empty_rounds += 1;
                
                // Wait for active tasks to complete
                if !active_tasks.is_empty() {
                    debug!("Waiting for {} active tasks to complete", active_tasks.len());
                    let completed_task = futures::future::select_all(active_tasks.drain(..)).await;
                    active_tasks = completed_task.2;
                } else {
                    // No active tasks and no pending URLs, we're done
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            consecutive_empty_rounds = 0;
            
            // Process batch concurrently
            let batch_tasks = self.process_batch(batch).await?;
            active_tasks.extend(batch_tasks);
            
            // Clean up completed tasks
            active_tasks.retain(|task| !task.is_finished());
            
            // Limit the number of concurrent tasks
            while active_tasks.len() >= self.config.max_concurrent {
                let completed_task = futures::future::select_all(active_tasks.drain(..)).await;
                active_tasks = completed_task.2;
            }
        }

        // Wait for all remaining tasks to complete
        if !active_tasks.is_empty() {
            info!("Waiting for {} remaining tasks to complete", active_tasks.len());
            futures::future::join_all(active_tasks).await;
        }

        // Stop monitoring
        if let Some(handle) = monitor_handle {
            handle.abort();
        }

        let duration = start_time.elapsed();
        let state = self.state.read().await;
        
        info!("🎉 Crawl completed in {:.2}s", duration.as_secs_f64());
        info!("📊 Results: {} processed, {} failed", state.total_processed, state.failed_urls.len());
        
        // Print final statistics
        self.print_final_stats(&state).await;
        
        Ok(())
    }

    async fn get_next_batch(&self) -> Vec<ParsedUrl> {
        let mut state = self.state.write().await;
        let mut batch = Vec::new();
        
        // Get up to max_concurrent URLs from the queue
        for _ in 0..self.config.max_concurrent {
            if let Some(url) = state.pending_urls.pop_front() {
                // Skip if already visited or failed
                if !state.visited_urls.contains(&url.url) && !state.failed_urls.contains(&url.url) {
                    // Check depth limit
                    if self.config.max_depth == 0 || url.depth <= self.config.max_depth {
                        batch.push(url);
                    }
                }
            } else {
                break;
            }
        }
        
        batch
    }

    async fn process_batch(&self, batch: Vec<ParsedUrl>) -> Result<Vec<JoinHandle<()>>> {
        let mut tasks = Vec::new();
        
        for url in batch {
            let task = self.process_url(url).await?;
            tasks.push(task);
        }
        
        Ok(tasks)
    }

    async fn process_url(&self, parsed_url: ParsedUrl) -> Result<JoinHandle<()>> {
        let crawler = self.clone();
        let semaphore = self.semaphore.clone();
        
        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            
            if let Err(e) = crawler.process_single_url(parsed_url).await {
                error!("Error processing URL: {}", e);
            }
        });
        
        Ok(task)
    }

    async fn process_single_url(&self, parsed_url: ParsedUrl) -> Result<()> {
        let url = &parsed_url.url;
        
        // Mark as visited
        {
            let mut state = self.state.write().await;
            state.visited_urls.insert(url.clone());
        }
        
        debug!("Processing URL: {} (depth: {})", url, parsed_url.depth);
        
        // Download the URL
        let download_result = self.downloader.download_url(url).await?;
        
        // Store result
        {
            let mut state = self.state.write().await;
            state.download_results.push(download_result.clone());
            state.total_processed += 1;
        }
        
        if !download_result.success {
            let mut state = self.state.write().await;
            state.failed_urls.insert(url.clone());
            warn!("Failed to download: {} - {}", url, download_result.error.unwrap_or_default());
            return Ok(());
        }
        
        // If it's an HTML page, parse it for more URLs
        if parsed_url.url_type == UrlType::Page {
            if let Some(content_type) = &download_result.content_type {
                if content_type.contains("text/html") {
                    if let Err(e) = self.parse_and_queue_urls(url, parsed_url.depth).await {
                        warn!("Failed to parse HTML from {}: {}", url, e);
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn parse_and_queue_urls(&self, url: &str, depth: usize) -> Result<()> {
        // Download HTML content for parsing
        let (download_result, content) = self.downloader.download_with_content(url).await?;
        
        if !download_result.success {
                            return Err(connection_error("Failed to download HTML content for parsing"));
        }
        
                        let content = content.ok_or_else(|| connection_error("No content received"))?;
        let html_content = String::from_utf8_lossy(&content);
        
        // Parse HTML to extract URLs
        let parsed_urls = self.parser.parse_html(&html_content, url, depth)?;
        
        // Add new URLs to queue
        {
            let mut state = self.state.write().await;
            let mut new_urls = 0;
            
            for parsed_url in parsed_urls {
                if !state.visited_urls.contains(&parsed_url.url) && 
                   !state.failed_urls.contains(&parsed_url.url) &&
                   !state.pending_urls.iter().any(|u| u.url == parsed_url.url) {
                    state.pending_urls.push_back(parsed_url);
                    new_urls += 1;
                }
            }
            
            state.total_found += new_urls;
            
            if new_urls > 0 {
                debug!("Found {} new URLs from {}", new_urls, url);
            }
        }
        
        Ok(())
    }

    async fn start_monitor(&self) -> Result<JoinHandle<()>> {
        let monitor = self.monitor.clone();
        let state = self.state.clone();
        let downloader = self.downloader.clone();
        
        let handle = tokio::spawn(async move {
            loop {
                let state_snapshot = state.read().await.clone();
                let download_stats = downloader.get_stats().await;
                
                monitor.update_stats(state_snapshot, download_stats).await;
                
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });
        
        Ok(handle)
    }

    async fn print_final_stats(&self, state: &CrawlState) {
        let download_stats = self.downloader.get_stats().await;
        
        info!("📈 Final Statistics:");
        info!("   URLs Found: {}", state.total_found);
        info!("   URLs Processed: {}", state.total_processed);
        info!("   URLs Failed: {}", state.failed_urls.len());
        info!("   Success Rate: {:.1}%", download_stats.success_rate());
        info!("   Bytes Downloaded: {}", format_bytes(download_stats.bytes_downloaded));
        info!("   Average Retries: {:.2}", download_stats.average_retries());
        
        // Print URL type breakdown
        let mut type_counts = HashMap::new();
        for result in &state.download_results {
            if result.success {
                let url_type = self.determine_result_type(result);
                *type_counts.entry(url_type).or_insert(0) += 1;
            }
        }
        
        info!("📁 Downloaded by type:");
        for (url_type, count) in type_counts {
            info!("   {}: {}", url_type, count);
        }
    }

    fn determine_result_type(&self, result: &DownloadResult) -> String {
        if let Some(content_type) = &result.content_type {
            if content_type.contains("text/html") {
                return "HTML Pages".to_string();
            } else if content_type.contains("image/") {
                return "Images".to_string();
            } else if content_type.contains("text/css") {
                return "CSS Files".to_string();
            } else if content_type.contains("application/javascript") || content_type.contains("text/javascript") {
                return "JavaScript Files".to_string();
            } else if content_type.contains("application/pdf") {
                return "PDF Documents".to_string();
            }
        }
        
        // Fallback to URL extension
        let url = Url::parse(&result.url).unwrap();
        let path = url.path().to_lowercase();
        
        if let Some(extension) = path.split('.').last() {
            match extension {
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico" => "Images".to_string(),
                "css" => "CSS Files".to_string(),
                "js" => "JavaScript Files".to_string(),
                "pdf" => "PDF Documents".to_string(),
                "zip" | "rar" | "7z" | "tar" | "gz" => "Archives".to_string(),
                _ => "Other Files".to_string(),
            }
        } else {
            "Other Files".to_string()
        }
    }
}

impl Clone for Crawler {
    fn clone(&self) -> Self {
        Crawler {
            config: self.config.clone(),
            downloader: self.downloader.clone(),
            parser: self.parser.clone(),
            monitor: self.monitor.clone(),
            state: self.state.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500.0 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[tokio::test]
    async fn test_crawler_creation() {
        let config = Config {
            url: "http://example.onion".to_string(),
            output_dir: PathBuf::from("./test_output"),
            max_depth: 2,
            max_concurrent: 4,
            delay_ms: 100,
            max_retries: 2,
            timeout_sec: 10,
            proxy: "127.0.0.1:9050".to_string(),
            user_agent: "TestAgent".to_string(),
            include_patterns: vec![],
            exclude_patterns: vec![],
            quiet: true,
            verbose: false,
            continue_download: false,
            dry_run: true,
            monitor: false,
        };

        let crawler = Crawler::new(config);
        assert!(crawler.is_ok());
    }
} 