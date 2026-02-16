use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::client::ClientInner;
use crate::error::Result;
use crate::types::{
    AccountHistoryEntry, AccountReputation, ActiveVote, AppliedOperation, BlockHeader,
    CollateralizedConversionRequest, Comment, Discussion, DiscussionQuery, DiscussionQueryCategory,
    DynamicGlobalProperties, Escrow, ExpiringVestingDelegation, ExtendedAccount, FeedHistory,
    FollowCount, FollowEntry, MarketBucket, MarketTrade, OpenOrder, OrderBook, OwnerHistory, Price,
    Proposal, RecoveryRequest, RecurrentTransfer, RewardFund, SavingsWithdraw, ScheduledHardfork,
    SignedBlock, SignedTransaction, Version, VestingDelegation, Witness,
};

#[derive(Debug, Clone)]
pub struct DatabaseApi {
    client: Arc<ClientInner>,
}

impl DatabaseApi {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        self.client.call("condenser_api", method, params).await
    }

    pub async fn get_accounts(&self, accounts: &[&str]) -> Result<Vec<ExtendedAccount>> {
        self.call("get_accounts", json!([accounts])).await
    }

    pub async fn get_account_count(&self) -> Result<u64> {
        self.call("get_account_count", json!([])).await
    }

    pub async fn get_account_history(
        &self,
        account: &str,
        start: i64,
        limit: u32,
    ) -> Result<Vec<AccountHistoryEntry>> {
        self.call("get_account_history", json!([account, start, limit]))
            .await
    }

    pub async fn get_account_reputations(
        &self,
        account_lower_bound: &str,
        limit: u32,
    ) -> Result<Vec<AccountReputation>> {
        self.call(
            "get_account_reputations",
            json!([account_lower_bound, limit]),
        )
        .await
    }

    pub async fn get_owner_history(&self, account: &str) -> Result<Vec<OwnerHistory>> {
        self.call("get_owner_history", json!([account])).await
    }

    pub async fn get_recovery_request(&self, account: &str) -> Result<Option<RecoveryRequest>> {
        self.call("get_recovery_request", json!([account])).await
    }

    pub async fn get_content(&self, author: &str, permlink: &str) -> Result<Comment> {
        self.call("get_content", json!([author, permlink])).await
    }

    pub async fn get_content_replies(&self, author: &str, permlink: &str) -> Result<Vec<Comment>> {
        self.call("get_content_replies", json!([author, permlink]))
            .await
    }

    pub async fn get_discussions(
        &self,
        by: DiscussionQueryCategory,
        query: &DiscussionQuery,
    ) -> Result<Vec<Discussion>> {
        let method = match by {
            DiscussionQueryCategory::Trending => "get_discussions_by_trending",
            DiscussionQueryCategory::Created => "get_discussions_by_created",
            DiscussionQueryCategory::Active => "get_discussions_by_active",
            DiscussionQueryCategory::Cashout => "get_discussions_by_cashout",
            DiscussionQueryCategory::Payout => "get_post_discussions_by_payout",
            DiscussionQueryCategory::Votes => "get_discussions_by_votes",
            DiscussionQueryCategory::Children => "get_discussions_by_children",
            DiscussionQueryCategory::Hot => "get_discussions_by_hot",
            DiscussionQueryCategory::Feed => "get_discussions_by_feed",
            DiscussionQueryCategory::Blog => "get_discussions_by_blog",
            DiscussionQueryCategory::Comments => "get_discussions_by_comments",
            DiscussionQueryCategory::Promoted => "get_discussions_by_promoted",
            DiscussionQueryCategory::Replies => "get_replies_by_last_update",
        };

        self.call(method, json!([query])).await
    }

    pub async fn get_discussions_by_author_before_date(
        &self,
        author: &str,
        start_permlink: &str,
        before_date: &str,
        limit: u32,
    ) -> Result<Vec<Discussion>> {
        self.call(
            "get_discussions_by_author_before_date",
            json!([author, start_permlink, before_date, limit]),
        )
        .await
    }

    pub async fn get_active_votes(&self, author: &str, permlink: &str) -> Result<Vec<ActiveVote>> {
        self.call("get_active_votes", json!([author, permlink]))
            .await
    }

    pub async fn get_dynamic_global_properties(&self) -> Result<DynamicGlobalProperties> {
        self.call("get_dynamic_global_properties", json!([])).await
    }

    pub async fn get_chain_properties(&self) -> Result<Value> {
        self.call("get_chain_properties", json!([])).await
    }

    pub async fn get_feed_history(&self) -> Result<FeedHistory> {
        self.call("get_feed_history", json!([])).await
    }

    pub async fn get_current_median_history_price(&self) -> Result<Price> {
        self.call("get_current_median_history_price", json!([]))
            .await
    }

    pub async fn get_hardfork_version(&self) -> Result<String> {
        self.call("get_hardfork_version", json!([])).await
    }

    pub async fn get_next_scheduled_hardfork(&self) -> Result<ScheduledHardfork> {
        self.call("get_next_scheduled_hardfork", json!([])).await
    }

    pub async fn get_reward_fund(&self, name: &str) -> Result<RewardFund> {
        self.call("get_reward_fund", json!([name])).await
    }

    pub async fn get_config(&self) -> Result<Value> {
        self.call("get_config", json!([])).await
    }

    pub async fn get_version(&self) -> Result<Version> {
        self.call("get_version", json!([])).await
    }

    pub async fn get_active_witnesses(&self) -> Result<Vec<String>> {
        self.call("get_active_witnesses", json!([])).await
    }

    pub async fn get_witness_by_account(&self, account: &str) -> Result<Option<Witness>> {
        self.call("get_witness_by_account", json!([account])).await
    }

    pub async fn get_vesting_delegations(
        &self,
        account: &str,
        from: &str,
        limit: u32,
    ) -> Result<Vec<VestingDelegation>> {
        self.call("get_vesting_delegations", json!([account, from, limit]))
            .await
    }

    pub async fn get_expiring_vesting_delegations(
        &self,
        account: &str,
        from: &str,
        limit: u32,
    ) -> Result<Vec<ExpiringVestingDelegation>> {
        self.call(
            "get_expiring_vesting_delegations",
            json!([account, from, limit]),
        )
        .await
    }

    pub async fn get_order_book(&self, limit: u32) -> Result<OrderBook> {
        self.call("get_order_book", json!([limit])).await
    }

    pub async fn get_open_orders(&self, account: &str) -> Result<Vec<OpenOrder>> {
        self.call("get_open_orders", json!([account])).await
    }

    pub async fn get_recent_trades(&self, limit: u32) -> Result<Vec<MarketTrade>> {
        self.call("get_recent_trades", json!([limit])).await
    }

    pub async fn get_market_history(
        &self,
        bucket_seconds: u32,
        start: &str,
        end: &str,
    ) -> Result<Vec<MarketBucket>> {
        self.call("get_market_history", json!([bucket_seconds, start, end]))
            .await
    }

    pub async fn get_market_history_buckets(&self) -> Result<Vec<u32>> {
        self.call("get_market_history_buckets", json!([])).await
    }

    pub async fn get_savings_withdraw_from(&self, account: &str) -> Result<Vec<SavingsWithdraw>> {
        self.call("get_savings_withdraw_from", json!([account]))
            .await
    }

    pub async fn get_savings_withdraw_to(&self, account: &str) -> Result<Vec<SavingsWithdraw>> {
        self.call("get_savings_withdraw_to", json!([account])).await
    }

    pub async fn get_conversion_requests(&self, account: &str) -> Result<Vec<Value>> {
        self.call("get_conversion_requests", json!([account])).await
    }

    pub async fn get_collateralized_conversion_requests(
        &self,
        account: &str,
    ) -> Result<Vec<CollateralizedConversionRequest>> {
        self.call("get_collateralized_conversion_requests", json!([account]))
            .await
    }

    pub async fn get_followers(
        &self,
        account: &str,
        start_follower: &str,
        follow_type: &str,
        limit: u32,
    ) -> Result<Vec<FollowEntry>> {
        self.call(
            "get_followers",
            json!([account, start_follower, follow_type, limit]),
        )
        .await
    }

    pub async fn get_following(
        &self,
        account: &str,
        start_following: &str,
        follow_type: &str,
        limit: u32,
    ) -> Result<Vec<FollowEntry>> {
        self.call(
            "get_following",
            json!([account, start_following, follow_type, limit]),
        )
        .await
    }

    pub async fn get_follow_count(&self, account: &str) -> Result<FollowCount> {
        self.call("get_follow_count", json!([account])).await
    }

    pub async fn get_reblogged_by(&self, author: &str, permlink: &str) -> Result<Vec<String>> {
        self.call("get_reblogged_by", json!([author, permlink]))
            .await
    }

    pub async fn get_blog(
        &self,
        account: &str,
        start_entry_id: u32,
        limit: u32,
    ) -> Result<Vec<Discussion>> {
        self.call("get_blog", json!([account, start_entry_id, limit]))
            .await
    }

    pub async fn get_blog_entries(
        &self,
        account: &str,
        start_entry_id: u32,
        limit: u32,
    ) -> Result<Vec<Value>> {
        self.call("get_blog_entries", json!([account, start_entry_id, limit]))
            .await
    }

    pub async fn get_potential_signatures(
        &self,
        transaction: &SignedTransaction,
    ) -> Result<Vec<String>> {
        self.call("get_potential_signatures", json!([transaction]))
            .await
    }

    pub async fn get_required_signatures(
        &self,
        transaction: &SignedTransaction,
        available_keys: &[String],
    ) -> Result<Vec<String>> {
        self.call(
            "get_required_signatures",
            json!([transaction, available_keys]),
        )
        .await
    }

    pub async fn verify_authority(&self, transaction: &SignedTransaction) -> Result<bool> {
        self.call("verify_authority", json!([transaction])).await
    }

    pub async fn get_key_references(&self, keys: &[String]) -> Result<Vec<Vec<String>>> {
        self.call("get_key_references", json!([keys])).await
    }

    pub async fn get_escrow(&self, from: &str, escrow_id: u32) -> Result<Option<Escrow>> {
        self.call("get_escrow", json!([from, escrow_id])).await
    }

    pub async fn find_proposals(&self, proposal_ids: &[i64]) -> Result<Vec<Proposal>> {
        self.call("find_proposals", json!([proposal_ids])).await
    }

    pub async fn list_proposals(
        &self,
        start: Value,
        limit: u32,
        order_by: &str,
        order_direction: &str,
        status: &str,
    ) -> Result<Vec<Proposal>> {
        self.call(
            "list_proposals",
            json!([start, limit, order_by, order_direction, status]),
        )
        .await
    }

    pub async fn find_recurrent_transfers(&self, account: &str) -> Result<Vec<RecurrentTransfer>> {
        self.call("find_recurrent_transfers", json!([account]))
            .await
    }

    pub async fn get_ops_in_block(
        &self,
        block_num: u32,
        only_virtual: bool,
    ) -> Result<Vec<AppliedOperation>> {
        self.call("get_ops_in_block", json!([block_num, only_virtual]))
            .await
    }

    pub async fn get_block(&self, block_num: u32) -> Result<Option<SignedBlock>> {
        self.call("get_block", json!([block_num])).await
    }

    pub async fn get_block_header(&self, block_num: u32) -> Result<Option<BlockHeader>> {
        self.call("get_block_header", json!([block_num])).await
    }
}
