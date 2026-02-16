use serde::{Deserialize, Serialize};

use crate::types::Operation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Transaction {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    #[serde(default)]
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SignedTransaction {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    #[serde(default)]
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TransactionConfirmation {
    pub id: String,
    pub block_num: u32,
    pub trx_num: u32,
    pub expired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TransactionStatus {
    pub status: String,
}
