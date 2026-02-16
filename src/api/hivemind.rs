use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::client::ClientInner;
use crate::error::Result;
use crate::types::{
    AccountNotifsQuery, AccountPostsQuery, CommunityDetail, CommunityQuery, CommunityRole,
    Discussion, ListCommunitiesQuery, Notification, PostsQuery,
};

#[derive(Debug, Clone)]
pub struct HivemindApi {
    client: Arc<ClientInner>,
}

impl HivemindApi {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        self.client.call("bridge", method, params).await
    }

    pub async fn get_ranked_posts(&self, query: &PostsQuery) -> Result<Vec<Discussion>> {
        self.call("get_ranked_posts", json!([query])).await
    }

    pub async fn get_account_posts(&self, query: &AccountPostsQuery) -> Result<Vec<Discussion>> {
        self.call("get_account_posts", json!([query])).await
    }

    pub async fn get_community(&self, query: &CommunityQuery) -> Result<CommunityDetail> {
        self.call("get_community", json!([query])).await
    }

    pub async fn list_communities(
        &self,
        query: &ListCommunitiesQuery,
    ) -> Result<Vec<CommunityDetail>> {
        self.call("list_communities", json!([query])).await
    }

    pub async fn get_community_roles(
        &self,
        community: &str,
        last: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<CommunityRole>> {
        self.call("get_community_roles", json!([community, last, limit]))
            .await
    }

    pub async fn get_account_notifications(
        &self,
        query: &AccountNotifsQuery,
    ) -> Result<Vec<Notification>> {
        self.call("get_account_notifications", json!([query])).await
    }

    pub async fn get_discussion(&self, author: &str, permlink: &str) -> Result<Discussion> {
        self.call("get_discussion", json!([author, permlink])).await
    }

    pub async fn get_post(&self, author: &str, permlink: &str) -> Result<Discussion> {
        self.call("get_post", json!([author, permlink])).await
    }
}
