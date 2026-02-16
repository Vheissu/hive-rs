use std::collections::BTreeMap;
use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum NumberLike {
    I64(i64),
    U64(u64),
    String(String),
}

impl NumberLike {
    fn to_i64(&self) -> std::result::Result<i64, String> {
        match self {
            Self::I64(value) => Ok(*value),
            Self::U64(value) => {
                i64::try_from(*value).map_err(|_| format!("value '{value}' exceeds i64 range"))
            }
            Self::String(value) => value
                .parse::<i64>()
                .map_err(|err| format!("invalid integer string '{value}': {err}")),
        }
    }

    fn to_u64(&self) -> std::result::Result<u64, String> {
        match self {
            Self::I64(value) => u64::try_from(*value)
                .map_err(|_| format!("value '{value}' cannot be represented as u64")),
            Self::U64(value) => Ok(*value),
            Self::String(value) => value
                .parse::<u64>()
                .map_err(|err| format!("invalid unsigned integer string '{value}': {err}")),
        }
    }

    fn to_u128(&self) -> std::result::Result<u128, String> {
        match self {
            Self::I64(value) => u128::try_from(*value)
                .map_err(|_| format!("value '{value}' cannot be represented as u128")),
            Self::U64(value) => Ok((*value).into()),
            Self::String(value) => value
                .parse::<u128>()
                .map_err(|err| format!("invalid unsigned integer string '{value}': {err}")),
        }
    }
}

fn deserialize_i64<'de, D>(deserializer: D) -> std::result::Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    NumberLike::deserialize(deserializer)?
        .to_i64()
        .map_err(D::Error::custom)
}

fn deserialize_u64<'de, D>(deserializer: D) -> std::result::Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    NumberLike::deserialize(deserializer)?
        .to_u64()
        .map_err(D::Error::custom)
}

fn deserialize_u128<'de, D>(deserializer: D) -> std::result::Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    NumberLike::deserialize(deserializer)?
        .to_u128()
        .map_err(D::Error::custom)
}

fn deserialize_opt_i64<'de, D>(deserializer: D) -> std::result::Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<NumberLike>::deserialize(deserializer)?
        .map(|value| value.to_i64())
        .transpose()
        .map_err(D::Error::custom)
}

fn deserialize_i64_map<'de, D>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = BTreeMap::<String, NumberLike>::deserialize(deserializer)?;
    raw.into_iter()
        .map(|(key, value)| value.to_i64().map(|parsed| (key, parsed)))
        .collect::<std::result::Result<BTreeMap<_, _>, _>>()
        .map_err(D::Error::custom)
}

fn deserialize_i64_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Vec::<NumberLike>::deserialize(deserializer)?;
    raw.into_iter()
        .map(|value| value.to_i64())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(D::Error::custom)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Manabar {
    #[serde(default, deserialize_with = "deserialize_i64")]
    pub current_mana: i64,
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub last_update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RCAccount {
    pub account: String,
    #[serde(default, deserialize_with = "deserialize_opt_i64")]
    pub delegated_rc: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_opt_i64")]
    pub max_rc: Option<i64>,
    #[serde(default)]
    pub rc_manabar: Option<Manabar>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RCPriceCurveParams {
    #[serde(default, deserialize_with = "deserialize_u128")]
    pub coeff_a: u128,
    #[serde(default, deserialize_with = "deserialize_u128")]
    pub coeff_b: u128,
    #[serde(default)]
    pub shift: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RCDecayParams {
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub decay_per_time_unit: u64,
    #[serde(default)]
    pub decay_per_time_unit_denom_shift: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RCDynamicsParams {
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub resource_unit: u64,
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub budget_per_time_unit: u64,
    #[serde(default, deserialize_with = "deserialize_i64")]
    pub pool_eq: i64,
    #[serde(default, deserialize_with = "deserialize_i64")]
    pub max_pool_size: i64,
    #[serde(default)]
    pub decay_params: RCDecayParams,
    #[serde(default, deserialize_with = "deserialize_i64")]
    pub min_decay: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RCResourceParam {
    #[serde(default)]
    pub resource_dynamics_params: RCDynamicsParams,
    #[serde(default)]
    pub price_curve_params: RCPriceCurveParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RCSizeInfo {
    #[serde(default, deserialize_with = "deserialize_i64_map")]
    pub resource_execution_time: BTreeMap<String, i64>,
    #[serde(default, deserialize_with = "deserialize_i64_map")]
    pub resource_state_bytes: BTreeMap<String, i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RCParams {
    #[serde(default)]
    pub resource_names: Vec<String>,
    #[serde(default)]
    pub resource_params: BTreeMap<String, RCResourceParam>,
    #[serde(default)]
    pub size_info: RCSizeInfo,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RCPoolResource {
    #[serde(default, deserialize_with = "deserialize_i64")]
    pub pool: i64,
    #[serde(default, deserialize_with = "deserialize_i64")]
    pub fill_level: i64,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RCPool {
    #[serde(default)]
    pub resource_pool: BTreeMap<String, RCPoolResource>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RcStats {
    #[serde(default, deserialize_with = "deserialize_i64")]
    pub regen: i64,
    #[serde(default, deserialize_with = "deserialize_i64_vec")]
    pub share: Vec<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl fmt::Display for RcStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "regen={}, share={:?}", self.regen, self.share)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::types::{RCAccount, RCParams, RCPool, RcStats};

    #[test]
    fn rc_account_parses_mixed_numeric_encodings() {
        let account: RCAccount = serde_json::from_value(json!({
            "account": "alice",
            "delegated_rc": 0,
            "max_rc": "135630143570",
            "rc_manabar": {
                "current_mana": "135375191366",
                "last_update_time": 1550731380
            }
        }))
        .expect("account should parse");

        assert_eq!(account.delegated_rc, Some(0));
        assert_eq!(account.max_rc, Some(135_630_143_570));
        assert_eq!(
            account.rc_manabar.as_ref().expect("manabar").current_mana,
            135_375_191_366
        );
    }

    #[test]
    fn rc_params_parse_with_resource_maps_and_size_info() {
        let params: RCParams = serde_json::from_value(json!({
            "resource_names": ["resource_history_bytes"],
            "resource_params": {
                "resource_history_bytes": {
                    "price_curve_params": {
                        "coeff_a": "10525659774662010880",
                        "coeff_b": 211332338,
                        "shift": 50
                    },
                    "resource_dynamics_params": {
                        "resource_unit": 1,
                        "budget_per_time_unit": 43403,
                        "pool_eq": 27050539251_i64,
                        "max_pool_size": "54101078501",
                        "decay_params": {
                            "decay_per_time_unit": 3613026481_u64,
                            "decay_per_time_unit_denom_shift": 51
                        },
                        "min_decay": 0
                    }
                }
            },
            "size_info": {
                "resource_execution_time": {
                    "transaction_time": 6622
                },
                "resource_state_bytes": {
                    "transaction_base_size": "128"
                }
            }
        }))
        .expect("params should parse");

        let history = params
            .resource_params
            .get("resource_history_bytes")
            .expect("history params");
        assert_eq!(
            history.price_curve_params.coeff_a,
            10_525_659_774_662_010_880
        );
        assert_eq!(
            history.resource_dynamics_params.max_pool_size,
            54_101_078_501
        );
        assert_eq!(
            params.size_info.resource_execution_time["transaction_time"],
            6622
        );
        assert_eq!(
            params.size_info.resource_state_bytes["transaction_base_size"],
            128
        );
    }

    #[test]
    fn rc_pool_and_stats_parse() {
        let pool: RCPool = serde_json::from_value(json!({
            "resource_pool": {
                "resource_execution_time": {
                    "pool": 66199826375_i64,
                    "fill_level": "9558"
                }
            }
        }))
        .expect("pool should parse");
        assert_eq!(
            pool.resource_pool["resource_execution_time"].fill_level,
            9558
        );

        let stats: RcStats = serde_json::from_value(json!({
            "regen": "2298172681338",
            "share": [5028, "10000", 436, 2467, 2068]
        }))
        .expect("stats should parse");
        assert_eq!(stats.regen, 2_298_172_681_338);
        assert_eq!(stats.share[1], 10_000);
    }
}
