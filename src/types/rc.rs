use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Manabar {
    #[serde(default)]
    pub current_mana: i64,
    #[serde(default)]
    pub last_update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RCAccount {
    pub account: String,
    #[serde(default)]
    pub rc_manabar: Option<Manabar>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RCParams {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RCPool {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
