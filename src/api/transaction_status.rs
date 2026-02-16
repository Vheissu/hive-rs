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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::api::TransactionStatusApi;
    use crate::client::{ClientInner, ClientOptions};
    use crate::transport::{BackoffStrategy, FailoverTransport};

    #[tokio::test]
    async fn find_transaction_routes_to_transaction_status_api() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["transaction_status_api", "find_transaction", [{"transaction_id": "deadbeef"}]]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": { "status": "within_mempool" }
            })))
            .mount(&server)
            .await;

        let transport = Arc::new(
            FailoverTransport::new(
                &[server.uri()],
                Duration::from_secs(2),
                1,
                BackoffStrategy::default(),
            )
            .expect("transport should initialize"),
        );
        let inner = Arc::new(ClientInner::new(transport, ClientOptions::default()));
        let api = TransactionStatusApi::new(inner);

        let response = api
            .find_transaction("deadbeef")
            .await
            .expect("rpc should succeed");
        assert_eq!(response.status, "within_mempool");
    }
}
