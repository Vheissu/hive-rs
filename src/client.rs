use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::api::{
    AccountByKeyApi, Blockchain, BroadcastApi, DatabaseApi, HivemindApi, RcApi,
    TransactionStatusApi,
};
use crate::error::Result;
use crate::transport::{BackoffStrategy, FailoverTransport};
use crate::types::ChainId;

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub timeout: Duration,
    pub failover_threshold: u32,
    pub address_prefix: String,
    pub chain_id: ChainId,
    pub backoff: BackoffStrategy,
}

impl Default for ClientOptions {
    fn default() -> Self {
        #[cfg(feature = "testnet")]
        let chain_id = ChainId::testnet();

        #[cfg(not(feature = "testnet"))]
        let chain_id = ChainId::mainnet();

        Self {
            timeout: Duration::from_secs(10),
            failover_threshold: 3,
            address_prefix: "STM".to_string(),
            chain_id,
            backoff: BackoffStrategy::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ClientInner {
    transport: Arc<FailoverTransport>,
    options: ClientOptions,
}

impl ClientInner {
    pub(crate) fn new(transport: Arc<FailoverTransport>, options: ClientOptions) -> Self {
        Self { transport, options }
    }

    pub(crate) async fn call<T: DeserializeOwned>(
        &self,
        api: &str,
        method: &str,
        params: Value,
    ) -> Result<T> {
        self.transport.call(api, method, params).await
    }

    pub(crate) fn options(&self) -> &ClientOptions {
        &self.options
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<ClientInner>,

    pub database: DatabaseApi,
    pub broadcast: BroadcastApi,
    pub blockchain: Blockchain,
    pub hivemind: HivemindApi,
    pub rc: RcApi,
    pub keys: AccountByKeyApi,
    pub transaction: TransactionStatusApi,
}

impl Client {
    pub fn new(nodes: Vec<&str>, options: ClientOptions) -> Self {
        let node_urls = nodes.into_iter().map(str::to_string).collect::<Vec<_>>();
        assert!(!node_urls.is_empty(), "at least one node URL is required");

        let transport = Arc::new(
            FailoverTransport::new(
                &node_urls,
                options.timeout,
                options.failover_threshold,
                options.backoff.clone(),
            )
            .expect("failed to initialize transport"),
        );

        let inner = Arc::new(ClientInner::new(transport, options));

        Self {
            database: DatabaseApi::new(inner.clone()),
            broadcast: BroadcastApi::new(inner.clone()),
            blockchain: Blockchain::new(inner.clone()),
            hivemind: HivemindApi::new(inner.clone()),
            rc: RcApi::new(inner.clone()),
            keys: AccountByKeyApi::new(inner.clone()),
            transaction: TransactionStatusApi::new(inner.clone()),
            inner,
        }
    }

    pub fn new_default() -> Self {
        Self::new(
            vec!["https://api.hive.blog", "https://api.openhive.network"],
            ClientOptions::default(),
        )
    }

    pub fn options(&self) -> &ClientOptions {
        self.inner.options()
    }

    pub async fn call<T: DeserializeOwned>(
        &self,
        api: &str,
        method: &str,
        params: Value,
    ) -> Result<T> {
        self.inner.call(api, method, params).await
    }
}
