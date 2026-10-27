use std::fmt;

#[derive(Debug)]
pub enum BlackskyError {
    NetworkError(String),
    HexDecodeError(String),
    RpcResponseError(String),
}

impl fmt::Display for BlackskyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlackskyError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            BlackskyError::HexDecodeError(msg) => write!(f, "Hex decode error: {}", msg),
            BlackskyError::RpcResponseError(msg) => write!(f, "RPC response error: {}", msg),
        }
    }
}

impl std::error::Error for BlackskyError {}
