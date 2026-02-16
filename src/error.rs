use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HiveError {
    #[error("RPC error {code}: {message}")]
    Rpc {
        code: i64,
        message: String,
        data: Option<Value>,
    },

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Signing error: {0}")]
    Signing(String),

    #[error("All nodes failed")]
    AllNodesFailed,

    #[error("Request timed out")]
    Timeout,

    #[error("Invalid asset: {0}")]
    InvalidAsset(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, HiveError>;

impl From<reqwest::Error> for HiveError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            Self::Timeout
        } else {
            Self::Transport(value.to_string())
        }
    }
}

impl From<serde_json::Error> for HiveError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::HiveError;

    #[test]
    fn error_variants_display() {
        let samples = vec![
            HiveError::Rpc {
                code: -32000,
                message: "boom".to_string(),
                data: None,
            },
            HiveError::Transport("io".to_string()),
            HiveError::Serialization("bad json".to_string()),
            HiveError::InvalidKey("bad key".to_string()),
            HiveError::Signing("failed".to_string()),
            HiveError::AllNodesFailed,
            HiveError::Timeout,
            HiveError::InvalidAsset("bad amount".to_string()),
            HiveError::Other("other".to_string()),
        ];

        for err in samples {
            assert!(!err.to_string().is_empty());
        }
    }
}
