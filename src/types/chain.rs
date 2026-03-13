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

    // Supply
    #[serde(default)]
    pub current_supply: Option<Asset>,
    #[serde(default)]
    pub current_hbd_supply: Option<Asset>,
    #[serde(default)]
    pub virtual_supply: Option<Asset>,
    #[serde(default)]
    pub total_reward_fund_hive: Option<Asset>,

    // Pending rewards
    #[serde(default)]
    pub pending_rewarded_vesting_shares: Option<Asset>,
    #[serde(default)]
    pub pending_rewarded_vesting_hive: Option<Asset>,

    // Rates & limits
    #[serde(default)]
    pub hbd_interest_rate: Option<u32>,
    #[serde(default)]
    pub hbd_print_rate: Option<u32>,
    #[serde(default)]
    pub maximum_block_size: Option<u32>,

    // Slots & participation
    #[serde(default)]
    pub current_aslot: Option<u64>,
    #[serde(default)]
    pub participation_count: Option<u32>,
    #[serde(default)]
    pub last_confirmed_block_num: Option<u32>,

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
