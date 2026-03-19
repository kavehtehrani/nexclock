use std::fmt;

#[derive(Debug)]
pub enum NexClockError {
    Network(String),
    Parse(String),
}

impl fmt::Display for NexClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Network error: {msg}"),
            Self::Parse(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

impl std::error::Error for NexClockError {}

impl From<reqwest::Error> for NexClockError {
    fn from(err: reqwest::Error) -> Self {
        Self::Network(err.to_string())
    }
}

impl From<serde_json::Error> for NexClockError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err.to_string())
    }
}
