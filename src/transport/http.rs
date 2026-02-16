use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::error::{HiveError, Result};

#[derive(Debug, Clone)]
pub struct HttpTransport {
    client: reqwest::Client,
    node_url: String,
}

impl HttpTransport {
    pub fn new(node_url: impl Into<String>, timeout: Duration) -> Result<Self> {
        let client = reqwest::Client::builder().timeout(timeout).build()?;
        Ok(Self {
            client,
            node_url: node_url.into(),
        })
    }

    pub fn node_url(&self) -> &str {
        self.node_url.as_str()
    }

    pub async fn call<T: DeserializeOwned>(
        &self,
        api: &str,
        method: &str,
        params: Value,
    ) -> Result<T> {
        let payload = json!({
            "id": 0,
            "jsonrpc": "2.0",
            "method": "call",
            "params": [api, method, params],
        });

        let response = self
            .client
            .post(&self.node_url)
            .json(&payload)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(HiveError::Transport(format!(
                "node {} returned HTTP {}",
                self.node_url,
                response.status()
            )));
        }

        let body: Value = response.json().await?;

        if let Some(err) = body.get("error") {
            let code = err.get("code").and_then(Value::as_i64).unwrap_or(-32000);
            let message = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown rpc error")
                .to_string();
            let data = err.get("data").cloned();

            return Err(HiveError::Rpc {
                code,
                message,
                data,
            });
        }

        let value = body
            .get("result")
            .cloned()
            .ok_or_else(|| HiveError::Serialization("missing JSON-RPC result field".to_string()))?;

        serde_json::from_value(value).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde::Deserialize;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::error::HiveError;
    use crate::transport::HttpTransport;

    #[derive(Debug, Deserialize)]
    struct OkResponse {
        ok: bool,
    }

    #[tokio::test]
    async fn sends_json_rpc_payload_and_parses_result() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "method": "call",
                "params": ["condenser_api", "get_config", []],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "ok": true
                }
            })))
            .mount(&server)
            .await;

        let transport = HttpTransport::new(server.uri(), Duration::from_secs(2))
            .expect("transport should initialize");

        let response: OkResponse = transport
            .call("condenser_api", "get_config", json!([]))
            .await
            .expect("request should succeed");

        assert!(response.ok);
    }

    #[tokio::test]
    async fn maps_rpc_error_payload_to_hive_error_rpc() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "error": {
                    "code": -32603,
                    "message": "boom",
                    "data": { "foo": "bar" }
                }
            })))
            .mount(&server)
            .await;

        let transport = HttpTransport::new(server.uri(), Duration::from_secs(2))
            .expect("transport should initialize");

        let err = transport
            .call::<serde_json::Value>("condenser_api", "get_config", json!([]))
            .await
            .expect_err("rpc response should be mapped to HiveError::Rpc");

        match err {
            HiveError::Rpc {
                code,
                message,
                data,
            } => {
                assert_eq!(code, -32603);
                assert_eq!(message, "boom");
                assert_eq!(data, Some(json!({ "foo": "bar" })));
            }
            other => panic!("expected HiveError::Rpc, got {other:?}"),
        }
    }
}
