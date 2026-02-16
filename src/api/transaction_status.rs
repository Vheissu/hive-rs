use std::sync::Arc;

use serde_json::json;

use crate::client::ClientInner;
use crate::error::Result;
use crate::types::TransactionStatus;

#[derive(Debug, Clone)]
pub struct TransactionStatusApi {
    client: Arc<ClientInner>,
}

impl TransactionStatusApi {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }

    pub async fn find_transaction(&self, transaction_id: &str) -> Result<TransactionStatus> {
        self.client
            .call(
                "transaction_status_api",
                "find_transaction",
                json!([{ "transaction_id": transaction_id }]),
            )
            .await
    }
}
