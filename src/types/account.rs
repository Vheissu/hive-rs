use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::Operation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExtendedAccount {
    pub name: String,
    #[serde(default)]
    pub reputation: Option<String>,
    #[serde(default)]
    pub memo_key: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AccountReputation {
    pub account: String,
    pub reputation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OwnerHistory {
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub previous_owner_authority: Option<Value>,
    #[serde(default)]
    pub last_valid_time: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RecoveryRequest {
    #[serde(default)]
    pub account_to_recover: Option<String>,
    #[serde(default)]
    pub recovery_account: Option<String>,
    #[serde(default)]
    pub new_owner_authority: Option<Value>,
    #[serde(default)]
    pub expires: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AccountHistoryEntry {
    pub index: u64,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub op: Option<Operation>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
