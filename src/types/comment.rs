use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::Asset;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Comment {
    pub author: String,
    pub permlink: String,
    #[serde(default)]
    pub parent_author: Option<String>,
    #[serde(default)]
    pub parent_permlink: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Discussion {
    #[serde(flatten)]
    pub comment: Comment,
    #[serde(default)]
    pub active_votes: Vec<ActiveVote>,
    #[serde(default)]
    pub pending_payout_value: Option<Asset>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BeneficiaryRoute {
    pub account: String,
    pub weight: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ActiveVote {
    pub voter: String,
    pub rshares: String,
    pub percent: i16,
    #[serde(default)]
    pub reputation: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscussionQueryCategory {
    Trending,
    Created,
    Active,
    Cashout,
    Payout,
    Votes,
    Children,
    Hot,
    Feed,
    Blog,
    Comments,
    Promoted,
    Replies,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DiscussionQuery {
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub start_author: Option<String>,
    #[serde(default)]
    pub start_permlink: Option<String>,
    #[serde(default)]
    pub truncate_body: Option<u32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

pub type DisqussionQuery = DiscussionQuery;
