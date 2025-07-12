use clap::{Arg, Command};
use env_logger::Env;
use log::{error, info};
use std::path::PathBuf;
use std::process;

mod config;
mod crawler;
mod downloader;
mod errors;
mod monitor;
mod parser;
mod tui;

use crate::config::Config;
use crate::crawler::Crawler;
use crate::errors::TorDownloaderError;
use crate::tui::TuiApp;

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse command line arguments
    let matches = Command::new("tor-downloader-rs")
        .version(env!("CARGO_PKG_VERSION"))
        .author("ZyraV21")
        .about("Multi-threaded recursive .onion site downloader with Tor support")
        .after_help("EXAMPLES:\n  \
            tor-downloader-rs --tui                    # Launch interactive TUI\n  \
            tor-downloader-rs http://example.onion     # Download with defaults\n  \
            tor-downloader-rs -j 16 -d 3 http://example.onion  # 16 jobs, depth 3\n\n\
            Created by ZyraV21 🚀")
        .arg(
            Arg::new("url")
                .help("The .onion URL to download (optional in TUI mode)")
                .required(false)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("DIR")
                .help("Output directory for downloaded files")
                .default_value("./downloaded_site"),
        )
        .arg(
            Arg::new("depth")
                .short('d')
                .long("depth")
                .value_name("NUM")
                .help("Maximum recursion depth (0 = infinite)")
                .default_value("5"),
        )
        .arg(
            Arg::new("jobs")
                .short('j')
                .long("jobs")
                .value_name("NUM")
                .help("Number of concurrent downloads")
                .default_value("8"),
        )
        .arg(
            Arg::new("delay")
                .short('D')
                .long("delay")
                .value_name("MS")
                .help("Delay between requests in milliseconds")
                .default_value("1000"),
        )
        .arg(
            Arg::new("retries")
                .short('r')
                .long("retries")
                .value_name("NUM")
                .help("Maximum number of retries per request")
                .default_value("3"),
        )
        .arg(
            Arg::new("timeout")
                .short('t')
                .long("timeout")
                .value_name("SEC")
                .help("Request timeout in seconds")
                .default_value("30"),
        )
        .arg(
            Arg::new("proxy")
                .short('p')
                .long("proxy")
                .value_name("PROXY")
                .help("SOCKS5 proxy address for Tor")
                .default_value("127.0.0.1:9050"),
        )
        .arg(
            Arg::new("user-agent")
                .short('u')
                .long("user-agent")
                .value_name("AGENT")
                .help("User agent string")
                .default_value("Mozilla/5.0 (Windows NT 10.0; rv:102.0) Gecko/20100101 Firefox/102.0"),
        )
        .arg(
            Arg::new("include")
                .short('i')
                .long("include")
                .value_name("REGEX")
                .help("Include URLs matching this regex pattern")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("exclude")
                .short('x')
                .long("exclude")
                .value_name("REGEX")
                .help("Exclude URLs matching this regex pattern")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress output except for errors")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("continue")
                .short('c')
                .long("continue")
                .help("Continue previous download")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("save-config")
                .long("save-config")
                .value_name("FILE")
                .help("Save configuration to file"),
        )
        .arg(
            Arg::new("load-config")
                .long("load-config")
                .value_name("FILE")
                .help("Load configuration from file"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Show what would be downloaded without downloading")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("monitor")
                .short('m')
                .long("monitor")
                .help("Enable real-time monitoring")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("tui")
                .long("tui")
                .help("Launch interactive TUI interface")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Check if TUI mode is requested
    if matches.get_flag("tui") {
        info!("Launching interactive TUI interface...");
        let mut app = TuiApp::new();
        if let Err(e) = app.run() {
            error!("TUI error: {}", e);
            process::exit(1);
        }
        return;
    }

    // Build configuration
    let config = match build_config(&matches) {
        Ok(config) => config,
        Err(e) => {
            error!("Configuration error: {}", e);
            process::exit(1);
        }
    };

    // Check if URL is required for CLI mode
    if config.url.is_empty() {
        error!("URL is required. Use --tui for interactive mode or provide a .onion URL.");
        process::exit(1);
    }

    // Save configuration if requested
    if let Some(config_file) = matches.get_one::<String>("save-config") {
        if let Err(e) = config.save_to_file(config_file) {
            error!("Failed to save configuration: {}", e);
            process::exit(1);
        }
        info!("Configuration saved to {}", config_file);
    }

    // Print configuration summary
    if !config.quiet {
        info!("🧅 Tor Downloader RS v{}", env!("CARGO_PKG_VERSION"));
        info!("📁 Target: {}", config.url);
        info!("💾 Output: {}", config.output_dir.display());
        info!("🔧 Jobs: {}, Depth: {}", config.max_concurrent, config.max_depth);
        info!("🌐 Proxy: {}", config.proxy);
        info!("⏱️  Delay: {}ms, Timeout: {}s", config.delay_ms, config.timeout_sec);
    }

    // Create and run crawler
    let crawler = match Crawler::new(config) {
        Ok(crawler) => crawler,
        Err(e) => {
            error!("Failed to create crawler: {}", e);
            process::exit(1);
        }
    };

    // Start crawling
    if let Err(e) = crawler.run().await {
        error!("Crawler error: {}", e);
        process::exit(1);
    }

    info!("🎉 Download completed successfully!");
}

fn build_config(matches: &clap::ArgMatches) -> Result<Config, TorDownloaderError> {
    let default_config = Config::default();
    let url = matches.get_one::<String>("url")
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_config.url.clone());
    let output_dir = PathBuf::from(matches.get_one::<String>("output").unwrap());
    let max_depth = matches.get_one::<String>("depth").unwrap().parse()?;
    let max_concurrent = matches.get_one::<String>("jobs").unwrap().parse()?;
    let delay_ms = matches.get_one::<String>("delay").unwrap().parse()?;
    let max_retries = matches.get_one::<String>("retries").unwrap().parse()?;
    let timeout_sec = matches.get_one::<String>("timeout").unwrap().parse()?;
    let proxy = matches.get_one::<String>("proxy").unwrap().to_string();
    let user_agent = matches.get_one::<String>("user-agent").unwrap().to_string();
    let quiet = matches.get_flag("quiet");
    let verbose = matches.get_flag("verbose");
    let continue_download = matches.get_flag("continue");
    let dry_run = matches.get_flag("dry-run");
    let monitor = matches.get_flag("monitor");

    let include_patterns = matches
        .get_many::<String>("include")
        .map(|vals| vals.map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let exclude_patterns = matches
        .get_many::<String>("exclude")
        .map(|vals| vals.map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let mut config = Config {
        url,
        output_dir,
        max_depth,
        max_concurrent,
        delay_ms,
        max_retries,
        timeout_sec,
        proxy,
        user_agent,
        include_patterns,
        exclude_patterns,
        quiet,
        verbose,
        continue_download,
        dry_run,
        monitor,
    };

    // Load configuration from file if specified
    if let Some(config_file) = matches.get_one::<String>("load-config") {
        config = Config::load_from_file(config_file)?;
    }

    // Validate configuration
    config.validate()?;

    Ok(config)
}
