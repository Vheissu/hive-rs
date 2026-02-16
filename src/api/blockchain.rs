use std::sync::Arc;

use crate::client::ClientInner;

#[derive(Debug, Clone)]
pub struct Blockchain {
    #[allow(dead_code)]
    client: Arc<ClientInner>,
}

impl Blockchain {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }
}
