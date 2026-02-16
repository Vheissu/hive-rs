use std::collections::BTreeMap;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::Operation;

fn deserialize_stringified<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(text) => Ok(text),
        Value::Number(number) => Ok(number.to_string()),
        other => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

fn deserialize_opt_stringified<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Value>::deserialize(deserializer)?
        .map(|value| match value {
            Value::String(text) => Ok(text),
            Value::Number(number) => Ok(number.to_string()),
            other => Err(D::Error::custom(format!(
                "expected string or number, got {other}"
            ))),
        })
        .transpose()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExtendedAccount {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_opt_stringified")]
    pub reputation: Option<String>,
    #[serde(default)]
    pub memo_key: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AccountReputation {
    pub account: String,
    #[serde(deserialize_with = "deserialize_stringified")]
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::types::{AccountReputation, ExtendedAccount};

    #[test]
    fn extended_account_supports_numeric_reputation() {
        let account: ExtendedAccount = serde_json::from_value(json!({
            "name": "beggars",
            "reputation": 0,
            "memo_key": "STM1111111111111111111111111111111114T1Anm",
        }))
        .expect("account should deserialize");

        assert_eq!(account.name, "beggars");
        assert_eq!(account.reputation.as_deref(), Some("0"));
    }

    #[test]
    fn account_reputation_supports_numeric_reputation() {
        let reputation: AccountReputation = serde_json::from_value(json!({
            "account": "alice",
            "reputation": 12345,
        }))
        .expect("reputation should deserialize");

        assert_eq!(reputation.account, "alice");
        assert_eq!(reputation.reputation, "12345");
    }
}
