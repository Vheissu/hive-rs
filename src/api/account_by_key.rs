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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::api::AccountByKeyApi;
    use crate::client::{ClientInner, ClientOptions};
    use crate::transport::{BackoffStrategy, FailoverTransport};

    #[tokio::test]
    async fn get_key_references_calls_expected_rpc_method() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["account_by_key_api", "get_key_references", [{"keys": ["STMabc"]}]]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": [["alice"]]
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
        let api = AccountByKeyApi::new(inner);

        let result = api
            .get_key_references(&["STMabc".to_string()])
            .await
            .expect("rpc should succeed");
        assert_eq!(result, vec![vec!["alice".to_string()]]);
    }
}
