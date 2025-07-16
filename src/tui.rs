use std::io::{stdout, Stdout};
use std::time::{Duration, Instant};
use std::process::Command;
use std::net::TcpStream;
use std::sync::Arc;
use tokio::sync::RwLock;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame, Terminal,
};

use crate::config::Config;
use crate::crawler::{Crawler, CrawlState};
use crate::errors::Result;

pub struct TuiApp {
    config: Config,
    state: AppState,
    list_state: ListState,
    input_mode: InputMode,
    current_input: String,
    editing_field: Option<ConfigField>,
    crawler: Option<Crawler>,
    crawler_task: Option<tokio::task::JoinHandle<()>>,
    crawler_state: Option<Arc<RwLock<CrawlState>>>,
    download_stats: DownloadStats,
    messages: Vec<String>,
    show_help: bool,
    show_confirm: bool,
    confirm_message: String,
    tor_status: TorStatus,
}

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    TorChecking,
    TorStarting,
    Configuration,
    Downloading,
    Paused,
    Completed,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Normal,
    Editing,
    Confirm,
}

#[derive(Debug, Clone, PartialEq)]
enum ConfigField {
    Url,
    OutputDir,
    MaxDepth,
    MaxConcurrent,
    DelayMs,
    MaxRetries,
    TimeoutSec,
    Proxy,
    UserAgent,
    IncludePattern,
    ExcludePattern,
}

#[derive(Debug, Clone, Default)]
struct DownloadStats {
    total_found: usize,
    total_processed: usize,
    successful: usize,
    failed: usize,
    bytes_downloaded: u64,
    current_url: String,
    progress: f64,
    speed: f64,
    eta: Duration,
}

#[derive(Debug, Clone)]
struct TorStatus {
    is_running: bool,
    port: u16,
    checking: bool,
    starting: bool,
    error: Option<String>,
    last_check: Instant,
    check_interval: Duration,
}

impl Default for TorStatus {
    fn default() -> Self {
        TorStatus {
            is_running: false,
            port: 9050,
            checking: false,
            starting: false,
            error: None,
            last_check: Instant::now(),
            check_interval: Duration::from_secs(2),
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        TuiApp {
            config: Config::default(),
            state: AppState::TorChecking,
            list_state,
            input_mode: InputMode::Normal,
            current_input: String::new(),
            editing_field: None,
            crawler: None,
            crawler_task: None,
            crawler_state: None,
            download_stats: DownloadStats::default(),
            messages: Vec::new(),
            show_help: false,
            show_confirm: false,
            confirm_message: String::new(),
            tor_status: TorStatus::default(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        // Start Tor checking process
        self.start_tor_check();
        
        // Create a tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new()?;
        let _guard = rt.enter();
        
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Ok(true) = event::poll(Duration::from_millis(100)) {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match self.input_mode {
                            InputMode::Normal => {
                                if self.handle_normal_input(key.code)? {
                                    break;
                                }
                            }
                            InputMode::Editing => {
                                if self.handle_editing_input(key.code)? {
                                    break;
                                }
                            }
                            InputMode::Confirm => {
                                if self.handle_confirm_input(key.code)? {
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // Update based on current state
            match self.state {
                AppState::TorChecking => {
                    self.update_tor_check();
                }
                AppState::TorStarting => {
                    self.update_tor_start();
                }
                AppState::Downloading => {
                    self.update_download_stats();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn start_tor_check(&mut self) {
        self.tor_status.checking = true;
        self.tor_status.last_check = Instant::now();
        self.messages.push("🔍 Checking Tor connection...".to_string());
    }

    fn update_tor_check(&mut self) {
        if !self.tor_status.checking {
            return;
        }

        // Only check if enough time has passed
        if self.tor_status.last_check.elapsed() < self.tor_status.check_interval {
            return;
        }

        self.tor_status.last_check = Instant::now();

        // Extract port from proxy config
        let port = self.config.proxy.split(':').nth(1)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(9050);
        
        self.tor_status.port = port;

        // Check if Tor is running on the specified port
        if self.is_tor_running(port) {
            self.tor_status.is_running = true;
            self.tor_status.checking = false;
            self.messages.push(format!("✅ Tor is running on port {}", port));
            self.state = AppState::Configuration;
        } else {
            self.tor_status.checking = false;
            self.tor_status.starting = true;
            self.tor_status.last_check = Instant::now(); // Reset timer for starting phase
            self.messages.push(format!("❌ Tor not running on port {}", port));
            self.messages.push("🚀 Attempting to start Tor...".to_string());
            self.state = AppState::TorStarting;
            self.start_tor_service(port);
        }
    }

    fn update_tor_start(&mut self) {
        if !self.tor_status.starting {
            return;
        }

        // Only check if enough time has passed
        if self.tor_status.last_check.elapsed() < self.tor_status.check_interval {
            return;
        }

        let elapsed_seconds = self.tor_status.last_check.elapsed().as_secs();
        self.tor_status.last_check = Instant::now();

        // Check if Tor has started
        if self.is_tor_running(self.tor_status.port) {
            self.tor_status.is_running = true;
            self.tor_status.starting = false;
            self.messages.push(format!("✅ Tor started successfully on port {}", self.tor_status.port));
            self.state = AppState::Configuration;
        } else {
            // Keep trying - add a message every few attempts to avoid spam
            if elapsed_seconds > 0 && elapsed_seconds % 6 == 0 {
                self.messages.push("⏳ Waiting for Tor to start...".to_string());
            }
        }
    }

    fn is_tor_running(&self, port: u16) -> bool {
        match TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn start_tor_service(&mut self, port: u16) {
        // Try to start Tor service with output suppressed
        let result = if cfg!(target_os = "windows") {
            // Windows: Try to start Tor Browser's tor.exe or system tor
            Command::new("tor.exe")
                .arg("--SocksPort")
                .arg(port.to_string())
                .arg("--DataDirectory")
                .arg("tor-data")
                .arg("--quiet")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
        } else {
            // Linux/macOS: Use system tor
            Command::new("tor")
                .arg("--SocksPort")
                .arg(port.to_string())
                .arg("--DataDirectory")
                .arg("tor-data")
                .arg("--quiet")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
        };

        match result {
            Ok(_) => {
                self.messages.push("🔄 Tor service started in background...".to_string());
            }
            Err(e) => {
                self.tor_status.error = Some(format!("Failed to start Tor: {}", e));
                self.messages.push(format!("❌ Failed to start Tor: {}", e));
                self.messages.push("ℹ️ Please ensure Tor is installed and accessible".to_string());
                self.state = AppState::Error(format!("Tor startup failed: {}", e));
            }
        }
    }

    fn handle_normal_input(&mut self, key: KeyCode) -> Result<bool> {
        match key {
            KeyCode::Char('q') => {
                if matches!(self.state, AppState::Downloading) {
                    self.show_confirm = true;
                    self.confirm_message = "Stop download and quit? (y/n)".to_string();
                    self.input_mode = InputMode::Confirm;
                } else {
                    return Ok(true);
                }
            }
            KeyCode::Char('h') | KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('r') => {
                match self.state {
                    AppState::Configuration => {
                        self.config = Config::default();
                        self.messages.push("Configuration reset to defaults".to_string());
                    }
                    AppState::Error(_) => {
                        // Reset and check Tor again
                        self.tor_status = TorStatus::default();
                        self.state = AppState::TorChecking;
                        self.start_tor_check();
                    }
                    _ => {}
                }
            }
            KeyCode::Char('s') => {
                if matches!(self.state, AppState::Configuration) {
                    self.start_download()?;
                }
            }
            KeyCode::Char('p') => {
                if matches!(self.state, AppState::Downloading) {
                    self.state = AppState::Paused;
                    self.messages.push("Download paused".to_string());
                } else if matches!(self.state, AppState::Paused) {
                    self.state = AppState::Downloading;
                    self.messages.push("Download resumed".to_string());
                }
            }
            KeyCode::Char('t') => {
                // Manual Tor recheck
                if matches!(self.state, AppState::Configuration | AppState::Error(_)) {
                    self.tor_status = TorStatus::default();
                    self.state = AppState::TorChecking;
                    self.start_tor_check();
                }
            }
            KeyCode::Up => {
                if matches!(self.state, AppState::Configuration) {
                    self.previous_config_item();
                }
            }
            KeyCode::Down => {
                if matches!(self.state, AppState::Configuration) {
                    self.next_config_item();
                }
            }
            KeyCode::Enter => {
                if matches!(self.state, AppState::Configuration) {
                    self.start_editing_selected_field();
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if matches!(self.state, AppState::Configuration) {
                    let index = c.to_digit(10).unwrap() as usize;
                    if index > 0 && index <= self.get_config_fields().len() {
                        self.list_state.select(Some(index - 1));
                        self.start_editing_selected_field();
                    }
                }
            }
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_editing_input(&mut self, key: KeyCode) -> Result<bool> {
        match key {
            KeyCode::Enter => {
                self.save_current_input();
                self.input_mode = InputMode::Normal;
                self.editing_field = None;
                self.current_input.clear();
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.editing_field = None;
                self.current_input.clear();
            }
            KeyCode::Char(c) => {
                self.current_input.push(c);
            }
            KeyCode::Backspace => {
                self.current_input.pop();
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_confirm_input(&mut self, key: KeyCode) -> Result<bool> {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_confirm = false;
                self.input_mode = InputMode::Normal;
                return Ok(true); // Quit application
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_confirm = false;
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
        Ok(false)
    }

    fn start_download(&mut self) -> Result<()> {
        self.messages.push("🔍 Checking download configuration...".to_string());
        
        if self.config.url.is_empty() {
            self.messages.push("❌ Error: URL is required".to_string());
            return Ok(());
        }

        self.messages.push(format!("📝 Target URL: {}", self.config.url));

        if let Err(e) = self.config.validate() {
            self.messages.push(format!("❌ Configuration error: {}", e));
            return Ok(());
        }

        self.messages.push("✅ Configuration validated successfully".to_string());
        self.state = AppState::Downloading;
        self.messages.push("🚀 Starting download...".to_string());

        // Create crawler
        match Crawler::new(self.config.clone()) {
            Ok(crawler) => {
                self.messages.push("✅ Crawler initialized successfully".to_string());
                self.messages.push("🔄 Starting real crawler execution...".to_string());
                
                // Get shared state from crawler
                let crawler_state = crawler.state.clone();
                self.crawler_state = Some(crawler_state);
                
                // Start crawler in background task
                let crawler_task = tokio::spawn(async move {
                    if let Err(e) = crawler.run().await {
                        eprintln!("Crawler error: {}", e);
                    }
                });
                
                self.crawler_task = Some(crawler_task);
                self.messages.push("🚀 Crawler started successfully".to_string());
                self.messages.push(format!("📍 Crawling: {}", self.config.url));
            }
            Err(e) => {
                self.state = AppState::Error(format!("Failed to initialize crawler: {}", e));
                self.messages.push(format!("❌ Crawler error: {}", e));
                return Ok(());
            }
        }

        Ok(())
    }

    fn update_download_stats(&mut self) {
        if let Some(ref crawler_state) = self.crawler_state {
            // Try to get stats from the crawler state
            if let Ok(state) = crawler_state.try_read() {
                    // Update stats from real crawler state
                    self.download_stats.total_found = state.total_found;
                    self.download_stats.total_processed = state.total_processed;
                    self.download_stats.successful = state.download_results.iter()
                        .filter(|r| r.success)
                        .count();
                    self.download_stats.failed = state.failed_urls.len();
                    
                    // Update progress
                    if self.download_stats.total_found > 0 {
                        self.download_stats.progress = 
                            (self.download_stats.total_processed as f64 / self.download_stats.total_found as f64) * 100.0;
                    }
                    
                    // Update current URL if we have pending URLs
                    if let Some(pending_url) = state.pending_urls.front() {
                        self.download_stats.current_url = pending_url.url.clone();
                    }
                    
                    // Check if crawler is done
                    if let Some(ref task) = self.crawler_task {
                        if task.is_finished() {
                            self.state = AppState::Completed;
                            self.messages.push("🎉 Download completed!".to_string());
                        }
                    }
            }
        }
    }

    fn get_config_fields(&self) -> Vec<ConfigField> {
        vec![
            ConfigField::Url,
            ConfigField::OutputDir,
            ConfigField::MaxDepth,
            ConfigField::MaxConcurrent,
            ConfigField::DelayMs,
            ConfigField::MaxRetries,
            ConfigField::TimeoutSec,
            ConfigField::Proxy,
            ConfigField::UserAgent,
            ConfigField::IncludePattern,
            ConfigField::ExcludePattern,
        ]
    }

    fn previous_config_item(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.get_config_fields().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn next_config_item(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.get_config_fields().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn start_editing_selected_field(&mut self) {
        if let Some(i) = self.list_state.selected() {
            let fields = self.get_config_fields();
            if let Some(field) = fields.get(i) {
                self.editing_field = Some(field.clone());
                self.current_input = self.get_field_value(field);
                self.input_mode = InputMode::Editing;
            }
        }
    }

    fn get_field_value(&self, field: &ConfigField) -> String {
        match field {
            ConfigField::Url => self.config.url.clone(),
            ConfigField::OutputDir => self.config.output_dir.to_string_lossy().to_string(),
            ConfigField::MaxDepth => self.config.max_depth.to_string(),
            ConfigField::MaxConcurrent => self.config.max_concurrent.to_string(),
            ConfigField::DelayMs => self.config.delay_ms.to_string(),
            ConfigField::MaxRetries => self.config.max_retries.to_string(),
            ConfigField::TimeoutSec => self.config.timeout_sec.to_string(),
            ConfigField::Proxy => self.config.proxy.clone(),
            ConfigField::UserAgent => self.config.user_agent.clone(),
            ConfigField::IncludePattern => self.config.include_patterns.join(", "),
            ConfigField::ExcludePattern => self.config.exclude_patterns.join(", "),
        }
    }

    fn save_current_input(&mut self) {
        if let Some(field) = &self.editing_field {
            match field {
                ConfigField::Url => {
                    self.config.url = self.current_input.clone();
                }
                ConfigField::OutputDir => {
                    self.config.output_dir = self.current_input.clone().into();
                }
                ConfigField::MaxDepth => {
                    if let Ok(val) = self.current_input.parse() {
                        self.config.max_depth = val;
                    }
                }
                ConfigField::MaxConcurrent => {
                    if let Ok(val) = self.current_input.parse() {
                        self.config.max_concurrent = val;
                    }
                }
                ConfigField::DelayMs => {
                    if let Ok(val) = self.current_input.parse() {
                        self.config.delay_ms = val;
                    }
                }
                ConfigField::MaxRetries => {
                    if let Ok(val) = self.current_input.parse() {
                        self.config.max_retries = val;
                    }
                }
                ConfigField::TimeoutSec => {
                    if let Ok(val) = self.current_input.parse() {
                        self.config.timeout_sec = val;
                    }
                }
                ConfigField::Proxy => {
                    self.config.proxy = self.current_input.clone();
                }
                ConfigField::UserAgent => {
                    self.config.user_agent = self.current_input.clone();
                }
                ConfigField::IncludePattern => {
                    self.config.include_patterns = self.current_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                ConfigField::ExcludePattern => {
                    self.config.exclude_patterns = self.current_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.size());

        self.render_header(f, chunks[0]);
        self.render_main_content(f, chunks[1]);
        self.render_footer(f, chunks[2]);

        if self.show_help {
            self.render_help_popup(f);
        }

        if self.show_confirm {
            self.render_confirm_popup(f);
        }
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let title = match self.state {
            AppState::TorChecking => "🔍 Tor Downloader - Checking Tor",
            AppState::TorStarting => "🚀 Tor Downloader - Starting Tor",
            AppState::Configuration => "🔧 Tor Downloader - Configuration",
            AppState::Downloading => "⬇️ Tor Downloader - Downloading",
            AppState::Paused => "⏸️ Tor Downloader - Paused",
            AppState::Completed => "✅ Tor Downloader - Completed",
            AppState::Error(_) => "❌ Tor Downloader - Error",
        };

        // Add Tor status indicator
        let tor_status = if self.tor_status.is_running {
            format!("Tor: ✅ :{}", self.tor_status.port)
        } else if self.tor_status.checking {
            "Tor: 🔍 Checking".to_string()
        } else if self.tor_status.starting {
            "Tor: 🚀 Starting".to_string()
        } else {
            "Tor: ❌ Not Running".to_string()
        };

        let header = Paragraph::new(title)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL)
                .title(format!("by ZyraV21 | {}", tor_status)));

        f.render_widget(header, area);
    }

    fn render_main_content(&mut self, f: &mut Frame, area: Rect) {
        match self.state {
            AppState::TorChecking => self.render_tor_checking(f, area),
            AppState::TorStarting => self.render_tor_starting(f, area),
            AppState::Configuration => self.render_configuration(f, area),
            AppState::Downloading | AppState::Paused => self.render_download_progress(f, area),
            AppState::Completed => self.render_completed(f, area),
            AppState::Error(ref error) => self.render_error(f, area, error),
        }
    }

    fn render_tor_checking(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        let text = format!(
            "🔍 Checking Tor Connection\n\n\
            Target Port: {}\n\
            Proxy Configuration: {}\n\n\
            Status: {}",
            self.tor_status.port,
            self.config.proxy,
            if self.tor_status.is_running {
                "✅ Tor is running"
            } else if self.tor_status.checking {
                "🔍 Checking connection..."
            } else {
                "❌ Tor is not running"
            }
        );

        let paragraph = Paragraph::new(text)
            .block(Block::default().title("Tor Status").borders(Borders::ALL))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[0]);

        // Messages panel
        let messages: Vec<ListItem> = self.messages
            .iter()
            .rev()
            .take(10)
            .map(|msg| ListItem::new(msg.clone()))
            .collect();

        let messages_list = List::new(messages)
            .block(Block::default().title("Messages").borders(Borders::ALL));

        f.render_widget(messages_list, chunks[1]);
    }

    fn render_tor_starting(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        let text = format!(
            "🚀 Starting Tor Service\n\n\
            Target Port: {}\n\
            Proxy Configuration: {}\n\n\
            Status: {}\n\n\
            {}",
            self.tor_status.port,
            self.config.proxy,
            if self.tor_status.is_running {
                "✅ Tor started successfully"
            } else if self.tor_status.starting {
                "⏳ Waiting for Tor to start..."
            } else {
                "❌ Tor failed to start"
            },
            if let Some(ref error) = self.tor_status.error {
                format!("Error: {}", error)
            } else {
                "Please wait while Tor initializes...".to_string()
            }
        );

        let paragraph = Paragraph::new(text)
            .block(Block::default().title("Tor Status").borders(Borders::ALL))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[0]);

        // Messages panel
        let messages: Vec<ListItem> = self.messages
            .iter()
            .rev()
            .take(10)
            .map(|msg| ListItem::new(msg.clone()))
            .collect();

        let messages_list = List::new(messages)
            .block(Block::default().title("Messages").borders(Borders::ALL));

        f.render_widget(messages_list, chunks[1]);
    }

    fn render_configuration(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Configuration list
        let config_items: Vec<ListItem> = self.get_config_fields()
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let (name, value) = match field {
                    ConfigField::Url => ("URL", self.config.url.clone()),
                    ConfigField::OutputDir => ("Output Directory", self.config.output_dir.to_string_lossy().to_string()),
                    ConfigField::MaxDepth => ("Max Depth", self.config.max_depth.to_string()),
                    ConfigField::MaxConcurrent => ("Max Concurrent", self.config.max_concurrent.to_string()),
                    ConfigField::DelayMs => ("Delay (ms)", self.config.delay_ms.to_string()),
                    ConfigField::MaxRetries => ("Max Retries", self.config.max_retries.to_string()),
                    ConfigField::TimeoutSec => ("Timeout (sec)", self.config.timeout_sec.to_string()),
                    ConfigField::Proxy => ("Proxy", self.config.proxy.clone()),
                    ConfigField::UserAgent => ("User Agent", self.config.user_agent.clone()),
                    ConfigField::IncludePattern => ("Include Patterns", self.config.include_patterns.join(", ")),
                    ConfigField::ExcludePattern => ("Exclude Patterns", self.config.exclude_patterns.join(", ")),
                };

                let display_value = if value.len() > 40 {
                    format!("{}...", &value[..37])
                } else {
                    value
                };

                let content = if let Some(ref editing) = self.editing_field {
                    if editing == field {
                        Line::from(vec![
                            Span::styled(format!("{}. {}: ", i + 1, name), Style::default().fg(Color::Yellow)),
                            Span::styled(&self.current_input, Style::default().fg(Color::Green)),
                            Span::styled("█", Style::default().fg(Color::Green)),
                        ])
                    } else {
                        Line::from(vec![
                            Span::styled(format!("{}. {}: ", i + 1, name), Style::default().fg(Color::White)),
                            Span::styled(display_value, Style::default().fg(Color::Gray)),
                        ])
                    }
                } else {
                    Line::from(vec![
                        Span::styled(format!("{}. {}: ", i + 1, name), Style::default().fg(Color::White)),
                        Span::styled(display_value, Style::default().fg(Color::Gray)),
                    ])
                };

                ListItem::new(content)
            })
            .collect();

        let config_list = List::new(config_items)
            .block(Block::default().title("Configuration").borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("► ");

        f.render_stateful_widget(config_list, chunks[0], &mut self.list_state);

        // Messages panel
        let messages: Vec<ListItem> = self.messages
            .iter()
            .rev()
            .take(20)
            .map(|msg| ListItem::new(msg.clone()))
            .collect();

        let messages_list = List::new(messages)
            .block(Block::default().title("Messages").borders(Borders::ALL));

        f.render_widget(messages_list, chunks[1]);
    }

    fn render_download_progress(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        // Progress bar
        let progress = Gauge::default()
            .block(Block::default().title("Overall Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(self.download_stats.progress as u16);

        f.render_widget(progress, chunks[0]);

        // Stats
        let stats_text = format!(
            "Found: {} | Processed: {} | Success: {} | Failed: {}",
            self.download_stats.total_found,
            self.download_stats.total_processed,
            self.download_stats.successful,
            self.download_stats.failed
        );

        let stats = Paragraph::new(stats_text)
            .block(Block::default().title("Statistics").borders(Borders::ALL));

        f.render_widget(stats, chunks[1]);

        // Current URL
        let current_url = Paragraph::new(self.download_stats.current_url.clone())
            .block(Block::default().title("Current URL").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(current_url, chunks[2]);

        // Speed and ETA
        let speed_text = format!(
            "Speed: {:.1} URLs/s | ETA: {}",
            self.download_stats.speed,
            format_duration(self.download_stats.eta)
        );

        let speed = Paragraph::new(speed_text)
            .block(Block::default().title("Performance").borders(Borders::ALL));

        f.render_widget(speed, chunks[3]);

        // Messages
        let messages: Vec<ListItem> = self.messages
            .iter()
            .rev()
            .take(10)
            .map(|msg| ListItem::new(msg.clone()))
            .collect();

        let messages_list = List::new(messages)
            .block(Block::default().title("Download Log").borders(Borders::ALL));

        f.render_widget(messages_list, chunks[4]);
    }

    fn render_completed(&self, f: &mut Frame, area: Rect) {
        let text = format!(
            "🎉 Download completed successfully!\n\n\
            Total URLs found: {}\n\
            Successfully downloaded: {}\n\
            Failed: {}\n\
            Bytes downloaded: {}\n\n\
            Press 'q' to quit or 'r' to start a new download.",
            self.download_stats.total_found,
            self.download_stats.successful,
            self.download_stats.failed,
            format_bytes(self.download_stats.bytes_downloaded)
        );

        let paragraph = Paragraph::new(text)
            .block(Block::default().title("Completed").borders(Borders::ALL))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    fn render_error(&self, f: &mut Frame, area: Rect, error: &str) {
        let text = format!(
            "❌ An error occurred:\n\n{}\n\n\
            Press 'q' to quit or 'r' to reset configuration.",
            error
        );

        let paragraph = Paragraph::new(text)
            .block(Block::default().title("Error").borders(Borders::ALL))
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.state {
            AppState::TorChecking => "T: Recheck Tor | Q: Quit",
            AppState::TorStarting => "T: Recheck Tor | Q: Quit",
            AppState::Configuration => {
                match self.input_mode {
                    InputMode::Editing => "Enter: Save | Esc: Cancel",
                    _ => "↑↓: Navigate | Enter/1-9: Edit | S: Start | T: Check Tor | R: Reset | H: Help | Q: Quit",
                }
            }
            AppState::Downloading => "P: Pause | Q: Quit | H: Help",
            AppState::Paused => "P: Resume | Q: Quit | H: Help",
            AppState::Completed | AppState::Error(_) => "R: New download | T: Check Tor | Q: Quit | H: Help",
        };

        let footer = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(footer, area);
    }

    fn render_help_popup(&self, f: &mut Frame) {
        let area = centered_rect(80, 60, f.size());

        let help_text = "🔧 Tor Downloader - Help\n\n\
        Navigation:\n\
        ↑↓ - Navigate configuration options\n\
        Enter - Edit selected option\n\
        1-9 - Quick edit option by number\n\n\
        Actions:\n\
        S - Start download\n\
        P - Pause/Resume download\n\
        R - Reset configuration\n\
        T - Check/Restart Tor connection\n\
        H - Toggle this help\n\
        Q - Quit application\n\n\
        Tor Status:\n\
        • The app automatically checks if Tor is running\n\
        • If Tor is not running, it attempts to start it\n\
        • Default port: 9050 (configurable in proxy setting)\n\
        • Use 'T' key to manually recheck Tor connection\n\n\
        Configuration:\n\
        • URL: The .onion site to download\n\
        • Output Directory: Where files will be saved\n\
        • Max Depth: How deep to crawl (0 = unlimited)\n\
        • Max Concurrent: Number of parallel downloads\n\
        • Delay: Milliseconds between requests\n\
        • Max Retries: Retry failed downloads\n\
        • Timeout: Request timeout in seconds\n\
        • Proxy: SOCKS5 proxy address\n\
        • User Agent: Browser identification\n\
        • Include/Exclude: Regex patterns for filtering\n\n\
        Press Esc to close this help.\n\n\
        Created by ZyraV21 🚀";

        let paragraph = Paragraph::new(help_text)
            .block(Block::default().title("Help").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }

    fn render_confirm_popup(&self, f: &mut Frame) {
        let area = centered_rect(50, 20, f.size());

        let paragraph = Paragraph::new(self.confirm_message.clone())
            .block(Block::default().title("Confirm").borders(Borders::ALL))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
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