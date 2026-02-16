use std::sync::Arc;

use serde_json::json;

use crate::client::ClientInner;
use crate::error::{HiveError, Result};
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
        match self
            .client
            .call(
                "transaction_status_api",
                "find_transaction",
                json!([{ "transaction_id": transaction_id }]),
            )
            .await
        {
            Ok(status) => Ok(status),
            Err(err) if should_fallback_to_condenser(&err) => {
                self.find_transaction_with_condenser(transaction_id).await
            }
            Err(err) => Err(err),
        }
    }

    async fn find_transaction_with_condenser(
        &self,
        transaction_id: &str,
    ) -> Result<TransactionStatus> {
        match self
            .client
            .call::<serde_json::Value>("condenser_api", "get_transaction", json!([transaction_id]))
            .await
        {
            Ok(_) => Ok(TransactionStatus {
                status: "found_in_block".to_string(),
            }),
            Err(HiveError::Rpc { message, .. }) if is_unknown_transaction_error(&message) => {
                Ok(TransactionStatus {
                    status: "unknown".to_string(),
                })
            }
            Err(err) => Err(err),
        }
    }
}

fn should_fallback_to_condenser(error: &HiveError) -> bool {
    let HiveError::Rpc { message, .. } = error else {
        return false;
    };

    let message = message.to_ascii_lowercase();
    message.contains("could not find method") || message.contains("could not find api")
}

fn is_unknown_transaction_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("unknown transaction")
        || message.contains("unable to find transaction")
        || message.contains("missing transaction")
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

    #[tokio::test]
    async fn falls_back_to_condenser_when_transaction_status_api_is_unavailable() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["transaction_status_api", "find_transaction", [{"transaction_id": "deadbeef"}]]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "error": {
                    "code": -32002,
                    "message": "Assert Exception: Could not find method find_transaction"
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["condenser_api", "get_transaction", ["deadbeef"]]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "transaction_id": "deadbeef",
                    "block_num": 99
                }
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
            .expect("fallback should succeed");
        assert_eq!(response.status, "found_in_block");
    }

    #[tokio::test]
    async fn fallback_reports_unknown_when_transaction_cannot_be_found() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["transaction_status_api", "find_transaction", [{"transaction_id": "deadbeef"}]]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "error": {
                    "code": -32002,
                    "message": "Assert Exception: Could not find method find_transaction"
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["condenser_api", "get_transaction", ["deadbeef"]]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "error": {
                    "code": -32003,
                    "message": "Unknown Transaction"
                }
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
            .expect("fallback should return unknown status");
        assert_eq!(response.status, "unknown");
    }
}
