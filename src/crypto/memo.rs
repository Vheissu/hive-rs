use crate::error::{HiveError, Result};

pub fn encode_memo(_message: &str) -> Result<String> {
    Err(HiveError::Other(
        "memo encryption is not implemented yet".to_string(),
    ))
}

pub fn decode_memo(_encoded: &str) -> Result<String> {
    Err(HiveError::Other(
        "memo decryption is not implemented yet".to_string(),
    ))
}
