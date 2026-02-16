use serde::{Deserialize, Serialize};

use crate::types::Asset;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Price {
    pub base: Asset,
    pub quote: Asset,
}
