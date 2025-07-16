use crate::errors::TorDownloaderError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub url: String,
    pub output_dir: PathBuf,
    pub max_depth: usize,
    pub max_concurrent: usize,
    pub delay_ms: u64,
    pub max_retries: usize,
    pub timeout_sec: u64,
    pub proxy: String,
    pub user_agent: String,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub quiet: bool,
    pub verbose: bool,
    pub continue_download: bool,
    pub dry_run: bool,
    pub monitor: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            url: "http://kxlpsf4uua2k36quvcob3mjlguurbc3rhjkwt7thoyi52o7y6tf2wrad.onion".to_string(),
            output_dir: PathBuf::from("./downloaded_site"),
            max_depth: 5,
            max_concurrent: 8,
            delay_ms: 1000,
            max_retries: 3,
            timeout_sec: 30,
            proxy: "127.0.0.1:9050".to_string(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; rv:102.0) Gecko/20100101 Firefox/102.0".to_string(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            quiet: false,
            verbose: false,
            continue_download: false,
            dry_run: false,
            monitor: false,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), TorDownloaderError> {
        // Validate URL
        let url = Url::parse(&self.url)?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(TorDownloaderError::InvalidUrl(
                "URL must use http or https scheme".to_string(),
            ));
        }

        // Check if it's a .onion URL
        if let Some(host) = url.host_str() {
            if !host.ends_with(".onion") {
                return Err(TorDownloaderError::InvalidUrl(
                    "URL must be a .onion address".to_string(),
                ));
            }
        } else {
            return Err(TorDownloaderError::InvalidUrl(
                "URL must have a valid host".to_string(),
            ));
        }

        // Validate concurrency limits
        if self.max_concurrent == 0 {
            return Err(TorDownloaderError::InvalidConfiguration(
                "max_concurrent must be greater than 0".to_string(),
            ));
        }

        if self.max_concurrent > 50 {
            return Err(TorDownloaderError::InvalidConfiguration(
                "max_concurrent should not exceed 50 to avoid overwhelming the server".to_string(),
            ));
        }

        // Validate delays
        if self.delay_ms > 60000 {
            return Err(TorDownloaderError::InvalidConfiguration(
                "delay_ms should not exceed 60000ms (1 minute)".to_string(),
            ));
        }

        if self.timeout_sec > 300 {
            return Err(TorDownloaderError::InvalidConfiguration(
                "timeout_sec should not exceed 300s (5 minutes)".to_string(),
            ));
        }

        // Validate proxy format
        if !self.proxy.contains(':') {
            return Err(TorDownloaderError::InvalidConfiguration(
                "proxy must be in format 'host:port'".to_string(),
            ));
        }

        Ok(())
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), TorDownloaderError> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self, TorDownloaderError> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }



    pub fn should_download_url(&self, url: &str) -> bool {
        // Check include patterns
        if !self.include_patterns.is_empty() {
            let matches_include = self.include_patterns.iter().any(|pattern| {
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(url))
                    .unwrap_or(false)
            });
            if !matches_include {
                return false;
            }
        }

        // Check exclude patterns
        if !self.exclude_patterns.is_empty() {
            let matches_exclude = self.exclude_patterns.iter().any(|pattern| {
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(url))
                    .unwrap_or(false)
            });
            if matches_exclude {
                return false;
            }
        }

        true
    }

    pub fn get_output_path(&self, url: &str) -> Result<PathBuf, TorDownloaderError> {
        let parsed_url = Url::parse(url)?;
        let mut path = self.output_dir.clone();

        // Add hostname
        if let Some(host) = parsed_url.host_str() {
            path.push(host);
        }

        // Add path segments
        let url_path = parsed_url.path();
        if url_path != "/" {
            let clean_path = url_path.trim_start_matches('/').trim_end_matches('/');
            if !clean_path.is_empty() {
                for segment in clean_path.split('/') {
                    if !segment.is_empty() {
                        // Clean segment name for filesystem compatibility
                        let clean_segment = segment
                            .chars()
                            .map(|c| {
                                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                                    c
                                } else {
                                    '_'
                                }
                            })
                            .collect::<String>();
                        path.push(clean_segment);
                    }
                }
            }
        }

        // If path ends with '/', add index.html
        if parsed_url.path().ends_with('/') {
            path.push("index.html");
        } else if path.is_dir() || !path.extension().is_some() {
            // If no extension, assume it's a directory and add index.html
            path.push("index.html");
        }

        Ok(path)
    }

    pub fn create_output_directory(&self) -> Result<(), TorDownloaderError> {
        if !self.output_dir.exists() {
            fs::create_dir_all(&self.output_dir)?;
        }
        Ok(())
    }
} 