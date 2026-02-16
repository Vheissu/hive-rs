use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{SignedTransaction, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BlockHeader {
    pub previous: String,
    pub timestamp: String,
    pub witness: String,
    pub transaction_merkle_root: String,
    #[serde(default)]
    pub extensions: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SignedBlockHeader {
    #[serde(flatten)]
    pub header: BlockHeader,
    pub witness_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SignedBlock {
    #[serde(flatten)]
    pub header: SignedBlockHeader,
    #[serde(default)]
    pub transactions: Vec<Transaction>,
    #[serde(default)]
    pub signed_transactions: Vec<SignedTransaction>,
    #[serde(default)]
    pub block_id: Option<String>,
    #[serde(default)]
    pub signing_key: Option<String>,
    #[serde(default)]
    pub transaction_ids: Vec<String>,
}
