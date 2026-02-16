use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::client::ClientInner;
use crate::error::Result;
use crate::types::{RCAccount, RCParams, RCPool};

#[derive(Debug, Clone)]
pub struct RcApi {
    client: Arc<ClientInner>,
}

impl RcApi {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        self.client.call("rc_api", method, params).await
    }

    pub async fn find_rc_accounts(&self, accounts: &[&str]) -> Result<Vec<RCAccount>> {
        self.call("find_rc_accounts", json!([{ "accounts": accounts }]))
            .await
    }

    pub async fn get_resource_params(&self) -> Result<RCParams> {
        self.call("get_resource_params", json!([{}])).await
    }

    pub async fn get_resource_pool(&self) -> Result<RCPool> {
        self.call("get_resource_pool", json!([{}])).await
    }

    pub fn calculate_cost(&self, _params: &RCParams, _pool: &RCPool) -> u64 {
        0
    }
}
