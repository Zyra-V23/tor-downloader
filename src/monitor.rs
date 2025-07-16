use crate::config::Config;
use crate::crawler::CrawlState;
use crate::downloader::DownloadStats;
use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
// use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct Monitor {
    config: Arc<Config>,
    start_time: Instant,
    progress_bar: Option<ProgressBar>,
    term: Term,
    last_stats: Arc<Mutex<MonitorStats>>,
}

#[derive(Debug, Clone)]
struct MonitorStats {
    total_found: usize,
    total_processed: usize,
    total_failed: usize,
    bytes_downloaded: u64,
    success_rate: f64,
    average_retries: f64,
    urls_per_second: f64,
    bytes_per_second: f64,
    last_update: Instant,
}

impl Default for MonitorStats {
    fn default() -> Self {
        MonitorStats {
            total_found: 0,
            total_processed: 0,
            total_failed: 0,
            bytes_downloaded: 0,
            success_rate: 0.0,
            average_retries: 0.0,
            urls_per_second: 0.0,
            bytes_per_second: 0.0,
            last_update: Instant::now(),
        }
    }
}

impl Monitor {
    pub fn new(config: Arc<Config>) -> Self {
        let term = Term::stdout();
        let start_time = Instant::now();
        
        let progress_bar = if !config.quiet && !config.dry_run {
            Some(create_progress_bar())
        } else {
            None
        };

        Monitor {
            config,
            start_time,
            progress_bar,
            term,
            last_stats: Arc::new(Mutex::new(MonitorStats {
                last_update: start_time,
                ..Default::default()
            })),
        }
    }

    pub async fn update_stats(&self, crawl_state: CrawlState, download_stats: DownloadStats) {
        let mut stats = self.last_stats.lock().await;
        let now = Instant::now();
        let time_diff = now.duration_since(stats.last_update).as_secs_f64();
        
        // Calculate rates
        let urls_diff = crawl_state.total_processed.saturating_sub(stats.total_processed);
        let bytes_diff = download_stats.bytes_downloaded.saturating_sub(stats.bytes_downloaded);
        
        let urls_per_second = if time_diff > 0.0 {
            urls_diff as f64 / time_diff
        } else {
            0.0
        };
        
        let bytes_per_second = if time_diff > 0.0 {
            bytes_diff as f64 / time_diff
        } else {
            0.0
        };
        
        // Update stats
        stats.total_found = crawl_state.total_found;
        stats.total_processed = crawl_state.total_processed;
        stats.total_failed = crawl_state.failed_urls.len();
        stats.bytes_downloaded = download_stats.bytes_downloaded;
        stats.success_rate = download_stats.success_rate();
        stats.average_retries = download_stats.average_retries();
        stats.urls_per_second = urls_per_second;
        stats.bytes_per_second = bytes_per_second;
        stats.last_update = now;
        
        // Update progress bar
        if let Some(ref pb) = self.progress_bar {
            pb.set_position(crawl_state.total_processed as u64);
            pb.set_length(crawl_state.total_found as u64);
            pb.set_message(format!(
                "Found: {} | Success: {:.1}% | Speed: {:.1}/s | Size: {}",
                crawl_state.total_found,
                stats.success_rate,
                urls_per_second,
                format_bytes(stats.bytes_downloaded)
            ));
        }
        
        // Print detailed stats if verbose and not quiet
        if self.config.verbose && !self.config.quiet {
            self.print_detailed_stats(&stats).await;
        }
    }

    async fn print_detailed_stats(&self, stats: &MonitorStats) {
        let elapsed = self.start_time.elapsed();
        
        if self.term.clear_screen().is_ok() {
            // Clear screen for real-time updates
        }
        
        println!("{}", style("🧅 Tor Downloader RS - Real-time Statistics").bold().cyan());
        println!("{}", style("═".repeat(60)).dim());
        
        println!("⏱️  {} | {} Elapsed", 
                style("Runtime:").bold(),
                format_duration(elapsed));
        
        println!("📊 {} | {} Found | {} Processed | {} Failed",
                style("Progress:").bold(),
                style(stats.total_found).green(),
                style(stats.total_processed).yellow(),
                style(stats.total_failed).red());
        
        println!("✅ {} | {:.1}% Success | {:.2} Avg Retries",
                style("Success:").bold(),
                style(format!("{:.1}", stats.success_rate)).green(),
                style(format!("{:.2}", stats.average_retries)).yellow());
        
        println!("🚀 {} | {:.1} URLs/s | {}/s",
                style("Speed:").bold(),
                style(format!("{:.1}", stats.urls_per_second)).cyan(),
                style(format_bytes(stats.bytes_per_second as u64)).cyan());
        
        println!("💾 {} | {} Downloaded",
                style("Data:").bold(),
                style(format_bytes(stats.bytes_downloaded)).magenta());
        
        let eta = if stats.urls_per_second > 0.0 {
            let remaining = stats.total_found.saturating_sub(stats.total_processed);
            let eta_seconds = remaining as f64 / stats.urls_per_second;
            format_duration(Duration::from_secs_f64(eta_seconds))
        } else {
            "Unknown".to_string()
        };
        
        println!("⏳ {} | {}",
                style("ETA:").bold(),
                style(eta).blue());
        
        println!("{}", style("═".repeat(60)).dim());
        
        // Memory usage (if available)
        if let Ok(usage) = get_memory_usage() {
            println!("🧠 {} | {} RAM",
                    style("Memory:").bold(),
                    style(format_bytes(usage)).dim());
        }
        
        println!("\n{}", style("Press Ctrl+C to stop").dim());
    }

    pub fn finish(&self) {
        if let Some(ref pb) = self.progress_bar {
            pb.finish_with_message("Download completed!");
        }
    }

    pub fn print_summary(&self, crawl_state: &CrawlState, download_stats: &DownloadStats) {
        let elapsed = self.start_time.elapsed();
        
        if !self.config.quiet {
            println!("\n{}", style("🎉 Download Summary").bold().green());
            println!("{}", style("═".repeat(50)).dim());
            
            println!("⏱️  Total Time: {}", format_duration(elapsed));
            println!("📊 URLs Found: {}", style(crawl_state.total_found).cyan());
            println!("✅ URLs Downloaded: {}", style(download_stats.successful_downloads).green());
            println!("❌ URLs Failed: {}", style(download_stats.failed_downloads).red());
            println!("📈 Success Rate: {:.1}%", style(download_stats.success_rate()).green());
            println!("💾 Data Downloaded: {}", style(format_bytes(download_stats.bytes_downloaded)).magenta());
            
            if elapsed.as_secs() > 0 {
                let avg_speed = download_stats.successful_downloads as f64 / elapsed.as_secs_f64();
                println!("🚀 Average Speed: {:.1} URLs/s", style(avg_speed).cyan());
            }
            
            if download_stats.retry_count > 0 {
                println!("🔄 Total Retries: {}", style(download_stats.retry_count).yellow());
                println!("📊 Avg Retries/URL: {:.2}", style(download_stats.average_retries()).yellow());
            }
        }
    }

    pub async fn print_progress_update(&self, current: usize, total: usize, message: &str) {
        if !self.config.quiet {
            let percentage = if total > 0 {
                (current as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            
            println!("📊 Progress: {}/{} ({:.1}%) - {}", 
                    style(current).cyan(),
                    style(total).cyan(),
                    style(percentage).yellow(),
                    message);
        }
    }

    pub fn log_error(&self, error: &str) {
        if !self.config.quiet {
            println!("❌ {}: {}", style("Error").red().bold(), error);
        }
    }

    pub fn log_warning(&self, warning: &str) {
        if !self.config.quiet {
            println!("⚠️  {}: {}", style("Warning").yellow().bold(), warning);
        }
    }

    pub fn log_info(&self, info: &str) {
        if !self.config.quiet {
            println!("ℹ️  {}: {}", style("Info").blue().bold(), info);
        }
    }

    pub fn log_success(&self, success: &str) {
        if !self.config.quiet {
            println!("✅ {}: {}", style("Success").green().bold(), success);
        }
    }
}

fn create_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
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

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn get_memory_usage() -> Result<u64, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("wmic")
            .args(&["process", "where", "name='tor-downloader-rs.exe'", "get", "WorkingSetSize", "/value"])
            .output()?;
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.starts_with("WorkingSetSize=") {
                let size_str = line.split('=').nth(1).unwrap_or("0").trim();
                if let Ok(size) = size_str.parse::<u64>() {
                    return Ok(size);
                }
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status")?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Ok(kb * 1024); // Convert KB to bytes
                    }
                }
            }
        }
    }
    
    Err("Memory usage not available".into())
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

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    }

    #[test]
    fn test_monitor_creation() {
        let config = Arc::new(Config {
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
        });

        let monitor = Monitor::new(config);
        assert!(monitor.config.quiet);
    }
} 