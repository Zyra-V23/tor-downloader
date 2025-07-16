//! # Tor Downloader RS
//! 
//! A fast, multi-threaded recursive .onion site downloader written in Rust.
//! 
//! This library provides components for downloading .onion sites through Tor
//! with advanced filtering, monitoring, and retry mechanisms.
//! 
//! ## Quick Start
//! 
//! ```rust,no_run
//! use tor_downloader_rs::config::Config;
//! use tor_downloader_rs::crawler::Crawler;
//! use std::path::PathBuf;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config {
//!         url: "http://example.onion".to_string(),
//!         output_dir: PathBuf::from("./downloads"),
//!         max_depth: 3,
//!         max_concurrent: 8,
//!         delay_ms: 1000,
//!         max_retries: 3,
//!         timeout_sec: 30,
//!         proxy: "127.0.0.1:9050".to_string(),
//!         user_agent: "Mozilla/5.0 (compatible; TorDownloaderRS/1.0)".to_string(),
//!         include_patterns: vec![],
//!         exclude_patterns: vec![],
//!         quiet: false,
//!         verbose: false,
//!         continue_download: false,
//!         dry_run: false,
//!         monitor: true,
//!     };
//! 
//!     let crawler = Crawler::new(config)?;
//!     crawler.run().await?;
//! 
//!     Ok(())
//! }
//! ```
//! 
//! ## Features
//! 
//! - **Multi-threaded**: Concurrent downloads with configurable limits
//! - **Tor-only**: All connections through SOCKS5 proxy
//! - **Smart Retry**: Exponential backoff with jitter
//! - **Real-time Monitoring**: Progress bars and live statistics
//! - **Intelligent Filtering**: Regex include/exclude patterns
//! - **File Type Detection**: Automatic categorization
//! - **Error Recovery**: Graceful handling of failures

pub mod config;
pub mod crawler;
pub mod downloader;
pub mod errors;
pub mod monitor;
pub mod parser;
pub mod tui;

// Re-export commonly used types
pub use config::Config;
pub use crawler::Crawler;
pub use downloader::{Downloader, DownloadResult, DownloadStats};
pub use errors::{Result, TorDownloaderError};
pub use monitor::Monitor;
pub use parser::{HtmlParser, ParsedUrl, UrlType};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration values
pub mod defaults {
    /// Default maximum recursion depth
    pub const MAX_DEPTH: usize = 5;
    
    /// Default number of concurrent downloads
    pub const MAX_CONCURRENT: usize = 8;
    
    /// Default delay between requests in milliseconds
    pub const DELAY_MS: u64 = 1000;
    
    /// Default maximum number of retries per request
    pub const MAX_RETRIES: usize = 3;
    
    /// Default request timeout in seconds
    pub const TIMEOUT_SEC: u64 = 30;
    
    /// Default Tor SOCKS5 proxy address
    pub const TOR_PROXY: &str = "127.0.0.1:9050";
    
    /// Default User-Agent string
    pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; rv:102.0) Gecko/20100101 Firefox/102.0";
    
    /// Default output directory
    pub const OUTPUT_DIR: &str = "./downloaded_site";
}

/// Utility functions
pub mod utils {
    /// Format bytes into human-readable format
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        format!("{:.1} {}", size, UNITS[unit_index])
    }
    
    /// Check if a URL is a valid .onion address
    pub fn is_onion_url(url: &str) -> bool {
        url::Url::parse(url)
            .map(|u| u.host_str().map_or(false, |h| h.ends_with(".onion")))
            .unwrap_or(false)
    }
    
    /// Clean filename for filesystem compatibility
    pub fn clean_filename(filename: &str) -> String {
        filename
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(utils::format_bytes(500), "500.0 B");
        assert_eq!(utils::format_bytes(1536), "1.5 KB");
        assert_eq!(utils::format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_is_onion_url() {
        assert!(utils::is_onion_url("http://example.onion"));
        assert!(utils::is_onion_url("https://test.onion/path"));
        assert!(!utils::is_onion_url("http://example.com"));
        assert!(!utils::is_onion_url("invalid-url"));
    }

    #[test]
    fn test_clean_filename() {
        assert_eq!(utils::clean_filename("file:name"), "file_name");
        assert_eq!(utils::clean_filename("file<>name"), "file__name");
        assert_eq!(utils::clean_filename("normal-file_name.txt"), "normal-file_name.txt");
    }
} 