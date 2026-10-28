#[allow(dead_code)]
use std::fmt;

#[derive(Debug)]
pub enum BlackskyError {
    NetworkError(String),
    HexDecodeError(String),
    RpcResponseError(String),
    StorageError(String),
    ValidationError(String),
}

impl From<std::io::Error> for BlackskyError {
    fn from(err: std::io::Error) -> Self {
        BlackskyError::StorageError(err.to_string())
    }
}

impl fmt::Display for BlackskyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlackskyError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            BlackskyError::HexDecodeError(msg) => write!(f, "Hex decode error: {}", msg),
            BlackskyError::RpcResponseError(msg) => write!(f, "RPC response error: {}", msg),
            BlackskyError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            BlackskyError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for BlackskyError {}

#[derive(Debug, Clone)]
pub struct SnapshotRecord {
    pub date: String,
    pub timestamp_utc: i64,
    pub chain: String,
    pub block_number: u64,
    pub address: String,
    pub category: String,
    pub label: String,
    pub is_liquid_overhang: bool,
    pub balance_geod: f64,
}
