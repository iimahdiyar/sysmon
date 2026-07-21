
use std::fmt;

#[derive(Debug)]
pub enum MonitorError {
    CollectionFailed(String),
    NetworkError(String),
    IoError(String),
    SerializationError(String),
    ProtocolError(String),
}

impl fmt::Display for MonitorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonitorError::CollectionFailed(msg) => write!(f, "Metric collection error: {msg}"),
            MonitorError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            MonitorError::IoError(msg) => write!(f, "File error: {msg}"),
            MonitorError::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            MonitorError::ProtocolError(msg) => write!(f, "Protocol error: {msg}"),
        }
    }
}

impl std::error::Error for MonitorError {}

impl From<std::io::Error> for MonitorError {
    fn from(e: std::io::Error) -> Self {
        MonitorError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for MonitorError {
    fn from(e: serde_json::Error) -> Self {
        MonitorError::SerializationError(e.to_string())
    }
}

pub type MonitorResult<T> = Result<T, MonitorError>;
