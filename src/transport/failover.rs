use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::error::{HiveError, Result};
use crate::transport::HttpTransport;

#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    Exponential { base_ms: u64, max_ms: u64 },
    Linear { step_ms: u64, max_ms: u64 },
    Fixed { ms: u64 },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::Exponential {
            base_ms: 100,
            max_ms: 10_000,
        }
    }
}

#[derive(Debug)]
struct FailoverState {
    current_index: usize,
    failures: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct FailoverTransport {
    transports: Vec<HttpTransport>,
    failover_threshold: u32,
    #[allow(dead_code)]
    backoff: BackoffStrategy,
    state: Arc<Mutex<FailoverState>>,
}

impl FailoverTransport {
    pub fn new(
        nodes: &[String],
        timeout: Duration,
        failover_threshold: u32,
        backoff: BackoffStrategy,
    ) -> Result<Self> {
        let mut transports = Vec::with_capacity(nodes.len());
        for node in nodes {
            transports.push(HttpTransport::new(node.clone(), timeout)?);
        }

        let failures = vec![0; transports.len()];
        Ok(Self {
            transports,
            failover_threshold: failover_threshold.max(1),
            backoff,
            state: Arc::new(Mutex::new(FailoverState {
                current_index: 0,
                failures,
            })),
        })
    }

    pub async fn call<T: DeserializeOwned>(
        &self,
        api: &str,
        method: &str,
        params: Value,
    ) -> Result<T> {
        if self.transports.is_empty() {
            return Err(HiveError::AllNodesFailed);
        }

        let start_index = self.state.lock().await.current_index;
        let mut last_error: Option<HiveError> = None;

        for offset in 0..self.transports.len() {
            let index = (start_index + offset) % self.transports.len();

            match self.transports[index]
                .call(api, method, params.clone())
                .await
            {
                Ok(result) => {
                    let mut state = self.state.lock().await;
                    state.current_index = index;
                    state.failures[index] = 0;
                    return Ok(result);
                }
                Err(HiveError::Rpc {
                    code,
                    message,
                    data,
                }) => {
                    return Err(HiveError::Rpc {
                        code,
                        message,
                        data,
                    })
                }
                Err(err) => {
                    last_error = Some(err);

                    let mut state = self.state.lock().await;
                    state.failures[index] = state.failures[index].saturating_add(1);
                    if state.failures[index] >= self.failover_threshold {
                        state.current_index = (index + 1) % self.transports.len();
                    }
                }
            }
        }

        Err(last_error.unwrap_or(HiveError::AllNodesFailed))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde::Deserialize;
    use serde_json::json;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::error::HiveError;
    use crate::transport::{BackoffStrategy, FailoverTransport};

    #[derive(Debug, Deserialize)]
    struct Ping {
        pong: bool,
    }

    #[tokio::test]
    async fn fails_over_to_next_node_when_first_node_is_unhealthy() {
        let first = MockServer::start().await;
        let second = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&first)
            .await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": { "pong": true }
            })))
            .mount(&second)
            .await;

        let transport = FailoverTransport::new(
            &[first.uri(), second.uri()],
            Duration::from_secs(2),
            1,
            BackoffStrategy::default(),
        )
        .expect("transport should initialize");

        let result: Ping = transport
            .call("condenser_api", "get_config", json!([]))
            .await
            .expect("second node should be used");

        assert!(result.pong);
    }

    #[tokio::test]
    async fn does_not_failover_on_rpc_error_response() {
        let first = MockServer::start().await;
        let second = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "error": {
                    "code": 10,
                    "message": "bad request"
                }
            })))
            .mount(&first)
            .await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": { "pong": true }
            })))
            .expect(0)
            .mount(&second)
            .await;

        let transport = FailoverTransport::new(
            &[first.uri(), second.uri()],
            Duration::from_secs(2),
            1,
            BackoffStrategy::default(),
        )
        .expect("transport should initialize");

        let err = transport
            .call::<Ping>("condenser_api", "get_config", json!([]))
            .await
            .expect_err("rpc error should be returned directly");

        match err {
            HiveError::Rpc { code, message, .. } => {
                assert_eq!(code, 10);
                assert_eq!(message, "bad request");
            }
            other => panic!("expected HiveError::Rpc, got {other:?}"),
        }
    }
}
