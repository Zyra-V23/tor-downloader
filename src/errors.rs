use thiserror::Error;

#[derive(Error, Debug)]
pub enum TorDownloaderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Proxy error: {0}")]
    Proxy(String),
    
    #[error("File system error: {0}")]
    FileSystem(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Rate limit error: {0}")]
    RateLimit(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Max retries exceeded: {0}")]
    MaxRetries(String),
    
    #[error("Operation cancelled")]
    Cancelled,
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, TorDownloaderError>;

pub fn validation_error(msg: &str) -> TorDownloaderError {
    TorDownloaderError::Validation(msg.to_string())
}

pub fn parse_error(msg: &str) -> TorDownloaderError {
    TorDownloaderError::Parse(msg.to_string())
}

pub fn connection_error(msg: &str) -> TorDownloaderError {
    TorDownloaderError::Connection(msg.to_string())
}

pub fn proxy_error(msg: &str) -> TorDownloaderError {
    TorDownloaderError::Proxy(msg.to_string())
}

pub fn network_error(msg: &str) -> TorDownloaderError {
    TorDownloaderError::Network(msg.to_string())
}

pub fn download_error(msg: &str) -> TorDownloaderError {
    TorDownloaderError::Network(msg.to_string())
}

pub fn parser_error(msg: &str) -> TorDownloaderError {
    TorDownloaderError::Parse(msg.to_string())
}

pub fn tor_connection_error(msg: &str) -> TorDownloaderError {
    TorDownloaderError::Connection(msg.to_string())
} 