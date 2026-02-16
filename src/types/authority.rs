use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Authority {
    pub weight_threshold: u32,
    #[serde(default)]
    pub account_auths: Vec<(String, u16)>,
    #[serde(default)]
    pub key_auths: Vec<(String, u16)>,
}
