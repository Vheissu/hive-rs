use std::sync::Arc;

use serde_json::json;

use crate::client::ClientInner;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct AccountByKeyApi {
    client: Arc<ClientInner>,
}

impl AccountByKeyApi {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }

    pub async fn get_key_references(&self, keys: &[String]) -> Result<Vec<Vec<String>>> {
        self.client
            .call(
                "account_by_key_api",
                "get_key_references",
                json!([{ "keys": keys }]),
            )
            .await
    }
}
