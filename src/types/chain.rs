use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{Asset, Price};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DynamicGlobalProperties {
    pub head_block_number: u32,
    pub head_block_id: String,
    pub time: String,
    #[serde(default)]
    pub current_witness: Option<String>,
    #[serde(default)]
    pub last_irreversible_block_num: u32,
    #[serde(default)]
    pub total_vesting_fund_hive: Option<Asset>,
    #[serde(default)]
    pub total_vesting_shares: Option<Asset>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainProperties {
    pub account_creation_fee: Asset,
    pub maximum_block_size: u32,
    pub hbd_interest_rate: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FeedHistory {
    #[serde(default)]
    pub current_median_history: Option<Price>,
    #[serde(default)]
    pub price_history: Vec<Price>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScheduledHardfork {
    pub hf_version: String,
    pub live_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RewardFund {
    #[serde(default)]
    pub id: Option<u32>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub reward_balance: Option<Asset>,
    #[serde(default)]
    pub recent_claims: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Version {
    pub blockchain_version: String,
    pub hive_revision: String,
    pub fc_revision: String,
}
