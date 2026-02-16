use std::sync::Arc;
use std::time::Duration;

use async_stream::try_stream;
use futures::Stream;
use serde_json::json;

use crate::client::ClientInner;
use crate::error::{HiveError, Result};
use crate::types::{AppliedOperation, BlockHeader, DynamicGlobalProperties, SignedBlock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlockchainMode {
    #[default]
    Irreversible,
    Latest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockchainStreamOptions {
    pub from: Option<u32>,
    pub to: Option<u32>,
    pub mode: BlockchainMode,
}

#[derive(Debug, Clone)]
pub struct Blockchain {
    client: Arc<ClientInner>,
}

impl Blockchain {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }

    pub async fn get_current_block_num(&self, mode: BlockchainMode) -> Result<u32> {
        let props: DynamicGlobalProperties = self
            .client
            .call("condenser_api", "get_dynamic_global_properties", json!([]))
            .await?;

        Ok(match mode {
            BlockchainMode::Irreversible => props.last_irreversible_block_num,
            BlockchainMode::Latest => props.head_block_number,
        })
    }

    pub async fn get_current_block_header(&self, mode: BlockchainMode) -> Result<BlockHeader> {
        let block_num = self.get_current_block_num(mode).await?;
        let header: Option<BlockHeader> = self
            .client
            .call("condenser_api", "get_block_header", json!([block_num]))
            .await?;

        header.ok_or_else(|| {
            HiveError::Serialization(format!("block header {block_num} not returned by node"))
        })
    }

    pub async fn get_current_block(&self, mode: BlockchainMode) -> Result<SignedBlock> {
        let block_num = self.get_current_block_num(mode).await?;
        let block: Option<SignedBlock> = self
            .client
            .call("condenser_api", "get_block", json!([block_num]))
            .await?;

        block.ok_or_else(|| {
            HiveError::Serialization(format!("block {block_num} not returned by node"))
        })
    }

    pub fn get_block_numbers(
        &self,
        options: BlockchainStreamOptions,
    ) -> impl Stream<Item = Result<u32>> + '_ {
        try_stream! {
            let interval = Duration::from_secs(3);
            let mut current = self.get_current_block_num(options.mode).await?;
            if let Some(from) = options.from {
                if from > current {
                    Err(HiveError::Other(format!(
                        "from cannot be larger than current block num ({current})"
                    )))?;
                }
            }

            let mut seen = options.from.unwrap_or(current);
            loop {
                while current > seen {
                    let next = seen;
                    seen = seen.saturating_add(1);
                    yield next;

                    if let Some(to) = options.to {
                        if seen > to {
                            return;
                        }
                    }
                }

                tokio::time::sleep(interval).await;
                current = self.get_current_block_num(options.mode).await?;
            }
        }
    }

    pub fn get_blocks(
        &self,
        options: BlockchainStreamOptions,
    ) -> impl Stream<Item = Result<SignedBlock>> + '_ {
        try_stream! {
            let numbers = self.get_block_numbers(options);
            futures::pin_mut!(numbers);

            while let Some(number_result) = futures::StreamExt::next(&mut numbers).await {
                let number = number_result?;
                let block: Option<SignedBlock> = self
                    .client
                    .call("condenser_api", "get_block", json!([number]))
                    .await?;
                if let Some(block) = block {
                    yield block;
                }
            }
        }
    }

    pub fn get_operations(
        &self,
        options: BlockchainStreamOptions,
    ) -> impl Stream<Item = Result<AppliedOperation>> + '_ {
        try_stream! {
            let numbers = self.get_block_numbers(options);
            futures::pin_mut!(numbers);

            while let Some(number_result) = futures::StreamExt::next(&mut numbers).await {
                let number = number_result?;
                let operations: Vec<AppliedOperation> = self
                    .client
                    .call("condenser_api", "get_ops_in_block", json!([number, false]))
                    .await?;
                for op in operations {
                    yield op;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::api::{Blockchain, BlockchainMode};
    use crate::client::{ClientInner, ClientOptions};
    use crate::transport::{BackoffStrategy, FailoverTransport};

    #[tokio::test]
    async fn current_block_num_uses_requested_mode() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "head_block_number": 100,
                    "head_block_id": "0000006400112233445566778899aabbccddeeff00112233445566778899aabb",
                    "time": "2024-01-01T00:00:00",
                    "last_irreversible_block_num": 95
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
        let blockchain = Blockchain::new(inner);

        let irreversible = blockchain
            .get_current_block_num(BlockchainMode::Irreversible)
            .await
            .expect("request should succeed");
        let latest = blockchain
            .get_current_block_num(BlockchainMode::Latest)
            .await
            .expect("request should succeed");

        assert_eq!(irreversible, 95);
        assert_eq!(latest, 100);
    }
}
