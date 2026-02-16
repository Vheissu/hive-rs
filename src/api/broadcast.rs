use std::sync::Arc;

use crate::client::ClientInner;

#[derive(Debug, Clone)]
pub struct BroadcastApi {
    #[allow(dead_code)]
    client: Arc<ClientInner>,
}

impl BroadcastApi {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }
}
