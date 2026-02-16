use std::sync::Arc;

use chrono::Utc;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::client::ClientInner;
use crate::error::{HiveError, Result};
use crate::serialization::serialize_transaction;
use crate::serialization::types::parse_hive_time;
use crate::types::{
    Authority, DynamicGlobalProperties, Operation, RCAccount, RCParams, RCPool, RCResourceParam,
    RcStats, Transaction,
};

const RESOURCE_HISTORY_BYTES: &str = "resource_history_bytes";
const RESOURCE_NEW_ACCOUNTS: &str = "resource_new_accounts";
const RESOURCE_MARKET_BYTES: &str = "resource_market_bytes";
const RESOURCE_STATE_BYTES: &str = "resource_state_bytes";
const RESOURCE_EXECUTION_TIME: &str = "resource_execution_time";
const KNOWN_RESOURCE_NAMES: [&str; 5] = [
    RESOURCE_HISTORY_BYTES,
    RESOURCE_NEW_ACCOUNTS,
    RESOURCE_MARKET_BYTES,
    RESOURCE_STATE_BYTES,
    RESOURCE_EXECUTION_TIME,
];

const RC_SHARE_BASIS_POINTS: i64 = 10_000;
const RC_REGEN_DIVISOR: i64 = 144_000;
const DEFAULT_SIGNATURE_COUNT: i64 = 1;
const SIGNATURE_SIZE_BYTES: i64 = 65;
const SIGNATURE_VECTOR_OVERHEAD_BYTES: i64 = 1;
const DEFAULT_EXPIRATION_HOURS: i64 = 1;

#[derive(Debug, Clone)]
pub struct RcApi {
    client: Arc<ClientInner>,
}

#[derive(Debug, Default, Clone, Copy)]
struct ResourceUsage {
    history_bytes: i64,
    new_accounts: i64,
    market_bytes: i64,
    state_bytes: i64,
    execution_time: i64,
}

impl ResourceUsage {
    fn by_name(self, resource_name: &str) -> i64 {
        match resource_name {
            RESOURCE_HISTORY_BYTES => self.history_bytes,
            RESOURCE_NEW_ACCOUNTS => self.new_accounts,
            RESOURCE_MARKET_BYTES => self.market_bytes,
            RESOURCE_STATE_BYTES => self.state_bytes,
            RESOURCE_EXECUTION_TIME => self.execution_time,
            _ => 0,
        }
    }
}

#[derive(Debug, Deserialize)]
struct FindRcAccountsResponse {
    #[serde(default)]
    rc_accounts: Vec<RCAccount>,
}

#[derive(Debug, Deserialize)]
struct RcStatsResponse {
    rc_stats: RcStats,
}

impl RcApi {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        self.client.call("rc_api", method, params).await
    }

    pub async fn find_rc_accounts(&self, accounts: &[&str]) -> Result<Vec<RCAccount>> {
        let response: FindRcAccountsResponse = self
            .call("find_rc_accounts", json!({ "accounts": accounts }))
            .await?;
        Ok(response.rc_accounts)
    }

    pub async fn get_resource_params(&self) -> Result<RCParams> {
        self.call("get_resource_params", json!({})).await
    }

    pub async fn get_resource_pool(&self) -> Result<RCPool> {
        self.call("get_resource_pool", json!({})).await
    }

    pub async fn calculate_cost(&self, operations: &[Operation]) -> Result<i64> {
        let params = self.get_resource_params().await?;
        let pool = self.get_resource_pool().await?;

        let (regen, shares) = match self.get_rc_stats().await {
            Ok(stats) if stats.regen > 0 => (stats.regen, share_map_from_stats(&params, &stats)),
            _ => {
                let regen = self.get_fallback_regen().await?;
                (regen, fallback_share_map(&params))
            }
        };

        calculate_cost_from_state(operations, &params, &pool, regen, &shares)
    }

    async fn get_rc_stats(&self) -> Result<RcStats> {
        let response: RcStatsResponse = self.call("get_rc_stats", json!({})).await?;
        Ok(response.rc_stats)
    }

    async fn get_fallback_regen(&self) -> Result<i64> {
        let props: DynamicGlobalProperties = self
            .client
            .call("condenser_api", "get_dynamic_global_properties", json!([]))
            .await?;

        let total_vests = props
            .total_vesting_shares
            .ok_or_else(|| HiveError::Other("missing total_vesting_shares".to_string()))?
            .amount;
        if total_vests <= 0 {
            return Err(HiveError::Other(
                "total_vesting_shares must be positive to estimate RC".to_string(),
            ));
        }
        Ok(total_vests / RC_REGEN_DIVISOR)
    }
}

fn calculate_cost_from_state(
    operations: &[Operation],
    params: &RCParams,
    pool: &RCPool,
    regen: i64,
    shares: &std::collections::BTreeMap<String, i64>,
) -> Result<i64> {
    if regen <= 0 {
        return Ok(0);
    }

    let usage = estimate_resource_usage(operations, params)?;
    let mut total_cost = 0_i64;
    for resource in ordered_resource_names(params) {
        let resource_name = resource.as_str();
        let resource_usage = usage.by_name(resource_name);
        if resource_usage <= 0 {
            continue;
        }

        let resource_params = params
            .resource_params
            .get(resource_name)
            .ok_or_else(|| HiveError::Other(format!("missing RC params for {resource_name}")))?;
        let resource_unit = i64::try_from(resource_params.resource_dynamics_params.resource_unit)
            .map_err(|_| {
            HiveError::Other(format!(
                "resource_unit for {resource_name} exceeds i64 range"
            ))
        })?;
        let scaled_usage = resource_usage.checked_mul(resource_unit).ok_or_else(|| {
            HiveError::Other(format!("scaled usage overflow for {resource_name}"))
        })?;
        let share_bp = shares.get(resource_name).copied().unwrap_or_default();
        let regen_share = pool_regen_share(regen, share_bp)?;
        if regen_share <= 0 {
            continue;
        }
        let pool_amount = pool
            .resource_pool
            .get(resource_name)
            .map(|entry| entry.pool)
            .unwrap_or(0);
        let resource_cost = compute_resource_cost(
            resource_params,
            pool_amount,
            scaled_usage,
            regen_share,
            resource_name,
        )?;
        total_cost = total_cost
            .checked_add(resource_cost)
            .ok_or_else(|| HiveError::Other("RC cost overflow".to_string()))?;
    }

    Ok(total_cost)
}

fn estimate_resource_usage(operations: &[Operation], params: &RCParams) -> Result<ResourceUsage> {
    let tx_size = estimate_signed_transaction_size(operations)?;

    let mut state_bytes = 0_i64;
    let mut execution_time = 0_i64;
    let mut new_account_ops = 0_i64;
    let mut market_op_count = 0_i64;

    for op in operations {
        match op {
            Operation::AccountCreate(op) => {
                state_bytes += state_size(params, "account_create_base_size")
                    + authority_dynamic_size(params, &op.owner)
                    + authority_dynamic_size(params, &op.active)
                    + authority_dynamic_size(params, &op.posting);
                execution_time += exec_time(params, "account_create_time");
            }
            Operation::AccountCreateWithDelegation(op) => {
                state_bytes += state_size(params, "account_create_base_size")
                    + authority_dynamic_size(params, &op.owner)
                    + authority_dynamic_size(params, &op.active)
                    + authority_dynamic_size(params, &op.posting)
                    + state_size(params, "vesting_delegation_object_size");
                execution_time += exec_time(params, "account_create_with_delegation_time");
            }
            Operation::AccountWitnessVote(op) => {
                if op.approve {
                    state_bytes += state_size(params, "account_witness_vote_size");
                }
                execution_time += exec_time(params, "account_witness_vote_time");
            }
            Operation::Comment(op) => {
                state_bytes += state_size(params, "comment_base_size")
                    + state_size(params, "comment_permlink_char_size") * op.permlink.len() as i64;
                execution_time += exec_time(params, "comment_time");
            }
            Operation::CommentOptions(op) => {
                for extension in &op.extensions {
                    match extension {
                        crate::types::CommentOptionsExtension::Beneficiaries { beneficiaries } => {
                            state_bytes += state_size(params, "comment_beneficiaries_member_size")
                                * beneficiaries.len() as i64;
                        }
                    }
                }
                execution_time += exec_time(params, "comment_options_time");
            }
            Operation::Convert(_) => {
                state_bytes += state_size(params, "convert_size");
                execution_time += exec_time(params, "convert_time");
            }
            Operation::CollateralizedConvert(_) => {
                state_bytes += state_size(params, "collateralized_convert_size");
                execution_time += exec_time(params, "collateralized_convert_time");
            }
            Operation::CreateClaimedAccount(op) => {
                state_bytes += state_size(params, "account_create_base_size")
                    + authority_dynamic_size(params, &op.owner)
                    + authority_dynamic_size(params, &op.active)
                    + authority_dynamic_size(params, &op.posting);
                execution_time += exec_time(params, "create_claimed_account_time");
            }
            Operation::DeclineVotingRights(op) => {
                if op.decline {
                    state_bytes += state_size(params, "decline_voting_rights_size");
                }
                execution_time += exec_time(params, "decline_voting_rights_time");
            }
            Operation::DelegateVestingShares(op) => {
                if op.vesting_shares.amount > 0 {
                    state_bytes += state_size(params, "delegate_vesting_shares_size");
                } else {
                    state_bytes += state_size(params, "vesting_delegation_expiration_object_size");
                }
                execution_time += exec_time(params, "delegate_vesting_shares_time");
            }
            Operation::EscrowTransfer(_) => {
                state_bytes += state_size(params, "escrow_transfer_size");
                execution_time += exec_time(params, "escrow_transfer_time");
            }
            Operation::LimitOrderCreate(op) => {
                if !op.fill_or_kill {
                    state_bytes += state_size(params, "limit_order_create_size");
                }
                execution_time += exec_time(params, "limit_order_create_time");
                market_op_count += 1;
            }
            Operation::LimitOrderCreate2(op) => {
                if !op.fill_or_kill {
                    state_bytes += state_size(params, "limit_order_create_size");
                }
                execution_time += exec_time(params, "limit_order_create2_time");
                market_op_count += 1;
            }
            Operation::RequestAccountRecovery(op) => {
                if op.new_owner_authority.weight_threshold != 0 {
                    state_bytes += state_size(params, "request_account_recovery_size");
                }
                execution_time += exec_time(params, "request_account_recovery_time");
            }
            Operation::SetWithdrawVestingRoute(op) => {
                if op.percent != 0 {
                    state_bytes += state_size(params, "set_withdraw_vesting_route_size");
                }
                execution_time += exec_time(params, "set_withdraw_vesting_route_time");
            }
            Operation::Vote(_) => {
                state_bytes += state_size(params, "vote_size");
                execution_time += exec_time(params, "vote_time");
            }
            Operation::WitnessUpdate(op) => {
                state_bytes += state_size(params, "witness_update_base_size")
                    + state_size(params, "witness_update_url_char_size") * op.url.len() as i64;
                execution_time += exec_time(params, "witness_update_time");
            }
            Operation::Transfer(_) => {
                execution_time += exec_time(params, "transfer_time");
                market_op_count += 1;
            }
            Operation::TransferToVesting(_) => {
                state_bytes += state_size(params, "transfer_to_vesting_size");
                execution_time += exec_time(params, "transfer_to_vesting_time");
                market_op_count += 1;
            }
            Operation::TransferToSavings(_) => {
                execution_time += exec_time(params, "transfer_to_savings_time");
            }
            Operation::TransferFromSavings(_) => {
                state_bytes += state_size(params, "transfer_from_savings_size");
                execution_time += exec_time(params, "transfer_from_savings_time");
            }
            Operation::ClaimRewardBalance(_) => {
                execution_time += exec_time(params, "claim_reward_balance_time");
            }
            Operation::WithdrawVesting(_) => {
                execution_time += exec_time(params, "withdraw_vesting_time");
            }
            Operation::AccountUpdate(op) => {
                if let Some(owner) = &op.owner {
                    state_bytes += authority_dynamic_size(params, owner)
                        + state_size(params, "owner_authority_history_object_size");
                }
                if let Some(active) = &op.active {
                    state_bytes += authority_dynamic_size(params, active);
                }
                if let Some(posting) = &op.posting {
                    state_bytes += authority_dynamic_size(params, posting);
                }
                execution_time += exec_time(params, "account_update_time");
            }
            Operation::AccountUpdate2(op) => {
                if let Some(owner) = &op.owner {
                    state_bytes += authority_dynamic_size(params, owner)
                        + state_size(params, "owner_authority_history_object_size");
                }
                if let Some(active) = &op.active {
                    state_bytes += authority_dynamic_size(params, active);
                }
                if let Some(posting) = &op.posting {
                    state_bytes += authority_dynamic_size(params, posting);
                }
                execution_time += exec_time(params, "account_update2_time");
            }
            Operation::AccountWitnessProxy(_) => {
                execution_time += exec_time(params, "account_witness_proxy_time");
            }
            Operation::CancelTransferFromSavings(_) => {
                execution_time += exec_time(params, "cancel_transfer_from_savings_time");
            }
            Operation::ChangeRecoveryAccount(_) => {
                state_bytes += state_size(params, "change_recovery_account_size");
                execution_time += exec_time(params, "change_recovery_account_time");
            }
            Operation::ClaimAccount(op) => {
                execution_time += exec_time(params, "claim_account_time");
                if op.fee.amount == 0 {
                    new_account_ops += 1;
                }
            }
            Operation::Custom(_) => {
                execution_time += exec_time(params, "custom_time");
            }
            Operation::CustomJson(_) => {
                execution_time += exec_time(params, "custom_json_time");
            }
            Operation::CustomBinary(_) => {
                execution_time += exec_time(params, "custom_binary_time");
            }
            Operation::DeleteComment(_) => {
                execution_time += exec_time(params, "delete_comment_time");
            }
            Operation::EscrowApprove(_) => {
                execution_time += exec_time(params, "escrow_approve_time");
            }
            Operation::EscrowDispute(_) => {
                execution_time += exec_time(params, "escrow_dispute_time");
            }
            Operation::EscrowRelease(_) => {
                execution_time += exec_time(params, "escrow_release_time");
            }
            Operation::FeedPublish(_) => {
                execution_time += exec_time(params, "feed_publish_time");
            }
            Operation::LimitOrderCancel(_) => {
                execution_time += exec_time(params, "limit_order_cancel_time");
            }
            Operation::WitnessSetProperties(_) => {
                execution_time += exec_time(params, "witness_set_properties_time");
            }
            Operation::CreateProposal(op) => {
                let lifetime_hours = proposal_lifetime_hours(&op.start_date, &op.end_date);
                let proposal_size = state_size(params, "create_proposal_base_size")
                    + state_size(params, "create_proposal_subject_permlink_char_size")
                        * (op.subject.len() + op.permlink.len()) as i64;
                state_bytes += proposal_size.saturating_mul(lifetime_hours);
                execution_time += exec_time(params, "create_proposal_time");
            }
            Operation::UpdateProposal(_) => {
                execution_time += exec_time(params, "update_proposal_time");
            }
            Operation::UpdateProposalVotes(op) => {
                if op.approve {
                    state_bytes += state_size(params, "update_proposal_votes_member_size")
                        * op.proposal_ids.len() as i64;
                }
                execution_time += exec_time(params, "update_proposal_votes_time");
            }
            Operation::RemoveProposal(_) => {
                execution_time += exec_time(params, "remove_proposal_time");
            }
            Operation::RecurrentTransfer(op) => {
                if op.amount.amount > 0 {
                    let lifetime = (op.recurrence as i64).saturating_mul(op.executions as i64);
                    state_bytes += (state_size(params, "recurrent_transfer_base_size")
                        + state_size(params, "recurrent_transfer_memo_char_size")
                            * op.memo.len() as i64)
                        .saturating_mul(lifetime);
                }
                execution_time += exec_time(params, "recurrent_transfer_base_time")
                    + exec_time(params, "recurrent_transfer_processing_time")
                        * op.executions as i64;
                market_op_count += 1;
            }
            Operation::RecoverAccount(op) => {
                state_bytes += authority_dynamic_size(params, &op.new_owner_authority)
                    + state_size(params, "owner_authority_history_object_size");
                execution_time += exec_time(params, "recover_account_time");
            }
            Operation::Pow(_)
            | Operation::Pow2(_)
            | Operation::ResetAccount(_)
            | Operation::SetResetAccount(_)
            | Operation::ReportOverProduction(_) => {}
        }
    }

    let transaction_base_size = state_size(params, "transaction_base_size");
    let transaction_time = exec_time(params, "transaction_time");
    let verify_authority_time = exec_time(params, "verify_authority_time");

    let usage = ResourceUsage {
        history_bytes: tx_size,
        new_accounts: new_account_ops,
        market_bytes: if market_op_count > 0 { tx_size } else { 0 },
        state_bytes: state_bytes + transaction_base_size.saturating_mul(DEFAULT_EXPIRATION_HOURS),
        execution_time: execution_time
            + transaction_time
            + verify_authority_time.saturating_mul(DEFAULT_SIGNATURE_COUNT),
    };

    Ok(usage)
}

fn estimate_signed_transaction_size(operations: &[Operation]) -> Result<i64> {
    let tx = Transaction {
        ref_block_num: 0,
        ref_block_prefix: 0,
        expiration: "1970-01-01T00:00:00".to_string(),
        operations: operations.to_vec(),
        extensions: Vec::new(),
    };

    let serialized = serialize_transaction(&tx)?;
    let tx_size = i64::try_from(serialized.len()).map_err(|_| {
        HiveError::Other("serialized transaction size exceeds i64 range".to_string())
    })?;
    Ok(tx_size + SIGNATURE_VECTOR_OVERHEAD_BYTES + SIGNATURE_SIZE_BYTES * DEFAULT_SIGNATURE_COUNT)
}

fn compute_resource_cost(
    params: &RCResourceParam,
    current_pool: i64,
    resource_count: i64,
    rc_regen: i64,
    resource_name: &str,
) -> Result<i64> {
    if rc_regen <= 0 || resource_count == 0 {
        return Ok(0);
    }
    if resource_count < 0 {
        return Ok(-compute_resource_cost(
            params,
            current_pool,
            -resource_count,
            rc_regen,
            resource_name,
        )?);
    }

    let mut numerator = u128::from(rc_regen as u64)
        .checked_mul(params.price_curve_params.coeff_a)
        .ok_or_else(|| HiveError::Other(format!("RC numerator overflow for {resource_name}")))?;
    numerator >>= params.price_curve_params.shift as usize;
    numerator = numerator.saturating_add(1);
    numerator = numerator
        .checked_mul(resource_count as u128)
        .ok_or_else(|| HiveError::Other(format!("RC numerator overflow for {resource_name}")))?;

    let pool_part = if current_pool > 0 {
        current_pool as u128
    } else {
        0
    };
    let denominator = params
        .price_curve_params
        .coeff_b
        .checked_add(pool_part)
        .ok_or_else(|| HiveError::Other(format!("RC denominator overflow for {resource_name}")))?;
    if denominator == 0 {
        return Err(HiveError::Other(format!(
            "RC denominator is zero for {resource_name}"
        )));
    }

    let quotient = numerator / denominator;
    let with_rounding = quotient.saturating_add(1);
    i64::try_from(with_rounding)
        .map_err(|_| HiveError::Other(format!("RC cost out of range for {resource_name}")))
}

fn pool_regen_share(regen: i64, share_basis_points: i64) -> Result<i64> {
    if regen <= 0 || share_basis_points <= 0 {
        return Ok(0);
    }
    let share =
        (i128::from(regen) * i128::from(share_basis_points)) / i128::from(RC_SHARE_BASIS_POINTS);
    i64::try_from(share).map_err(|_| HiveError::Other("regen share out of range".to_string()))
}

fn ordered_resource_names(params: &RCParams) -> Vec<String> {
    if !params.resource_names.is_empty() {
        return params.resource_names.clone();
    }

    let mut names = KNOWN_RESOURCE_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    for name in params.resource_params.keys() {
        if !names.iter().any(|known| known == name) {
            names.push(name.clone());
        }
    }
    names
}

fn share_map_from_stats(
    params: &RCParams,
    stats: &RcStats,
) -> std::collections::BTreeMap<String, i64> {
    let resource_names = ordered_resource_names(params);
    if stats.share.len() < resource_names.len() {
        return fallback_share_map(params);
    }

    resource_names
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.clone(), stats.share[idx].max(0)))
        .collect()
}

fn fallback_share_map(params: &RCParams) -> std::collections::BTreeMap<String, i64> {
    let resource_names = ordered_resource_names(params);
    let mut map = std::collections::BTreeMap::new();
    let non_new_names = resource_names
        .iter()
        .filter(|name| name.as_str() != RESOURCE_NEW_ACCOUNTS)
        .cloned()
        .collect::<Vec<_>>();

    let mut budget_sum = 0_i64;
    for name in &non_new_names {
        if let Some(resource) = params.resource_params.get(name) {
            let budget =
                i64::try_from(resource.resource_dynamics_params.budget_per_time_unit).unwrap_or(0);
            budget_sum = budget_sum.saturating_add(budget.max(0));
        }
    }

    let mut assigned = 0_i64;
    for (idx, name) in non_new_names.iter().enumerate() {
        let share = if budget_sum > 0 {
            let budget = params
                .resource_params
                .get(name)
                .and_then(|resource| {
                    i64::try_from(resource.resource_dynamics_params.budget_per_time_unit).ok()
                })
                .unwrap_or(0)
                .max(0);
            if idx + 1 == non_new_names.len() {
                RC_SHARE_BASIS_POINTS.saturating_sub(assigned)
            } else {
                let computed = (budget * RC_SHARE_BASIS_POINTS) / budget_sum;
                assigned = assigned.saturating_add(computed);
                computed
            }
        } else if non_new_names.is_empty() {
            0
        } else if idx + 1 == non_new_names.len() {
            RC_SHARE_BASIS_POINTS.saturating_sub(assigned)
        } else {
            let computed = RC_SHARE_BASIS_POINTS / non_new_names.len() as i64;
            assigned = assigned.saturating_add(computed);
            computed
        };
        map.insert(name.clone(), share.max(0));
    }

    map.insert(RESOURCE_NEW_ACCOUNTS.to_string(), RC_SHARE_BASIS_POINTS);
    map
}

fn state_size(params: &RCParams, key: &str) -> i64 {
    params
        .size_info
        .resource_state_bytes
        .get(key)
        .copied()
        .unwrap_or(0)
}

fn exec_time(params: &RCParams, key: &str) -> i64 {
    params
        .size_info
        .resource_execution_time
        .get(key)
        .copied()
        .unwrap_or(0)
}

fn authority_dynamic_size(params: &RCParams, authority: &Authority) -> i64 {
    state_size(params, "authority_account_member_size") * authority.account_auths.len() as i64
        + state_size(params, "authority_key_member_size") * authority.key_auths.len() as i64
}

fn proposal_lifetime_hours(start_date: &str, end_date: &str) -> i64 {
    let start = parse_hive_time(start_date).ok();
    let end = parse_hive_time(end_date).ok();
    if let (Some(start), Some(end)) = (start, end) {
        let diff = end.timestamp().saturating_sub(start.timestamp());
        if diff > 0 {
            return (diff + 3599) / 3600;
        }
    }

    let now = Utc::now();
    if let Some(end) = end {
        let diff = end.timestamp().saturating_sub(now.timestamp());
        if diff > 0 {
            return (diff + 3599) / 3600;
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::api::RcApi;
    use crate::client::{ClientInner, ClientOptions};
    use crate::transport::{BackoffStrategy, FailoverTransport};
    use crate::types::{Asset, Operation, RcStats, TransferOperation};

    #[tokio::test]
    async fn find_rc_accounts_uses_object_params_and_unwraps_result() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["rc_api", "find_rc_accounts", {"accounts": ["alice"]}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": { "rc_accounts": [{ "account": "alice", "max_rc": "1" }] }
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
        let api = RcApi::new(inner);

        let accounts = api
            .find_rc_accounts(&["alice"])
            .await
            .expect("rpc should succeed");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].account, "alice");
        assert_eq!(accounts[0].max_rc, Some(1));
    }

    #[tokio::test]
    async fn resource_methods_use_object_params() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["rc_api", "get_resource_params", {}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "resource_names": ["resource_history_bytes"],
                    "resource_params": {
                        "resource_history_bytes": {
                            "price_curve_params": { "coeff_a": "1", "coeff_b": "1", "shift": 0 },
                            "resource_dynamics_params": {
                                "resource_unit": 1,
                                "budget_per_time_unit": 1,
                                "pool_eq": 1,
                                "max_pool_size": 1,
                                "decay_params": { "decay_per_time_unit": 1, "decay_per_time_unit_denom_shift": 1 },
                                "min_decay": 0
                            }
                        }
                    },
                    "size_info": {
                        "resource_execution_time": { "transaction_time": 1, "verify_authority_time": 1 },
                        "resource_state_bytes": { "transaction_base_size": 1 }
                    }
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["rc_api", "get_resource_pool", {}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "resource_pool": {
                        "resource_history_bytes": { "pool": 1, "fill_level": 1 }
                    }
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
        let api = RcApi::new(inner);

        let params = api
            .get_resource_params()
            .await
            .expect("get_resource_params should succeed");
        assert_eq!(params.resource_names, vec!["resource_history_bytes"]);

        let pool = api
            .get_resource_pool()
            .await
            .expect("get_resource_pool should succeed");
        assert_eq!(pool.resource_pool["resource_history_bytes"].pool, 1);
    }

    #[tokio::test]
    async fn calculate_cost_uses_live_formula_with_stats_share() {
        let server = MockServer::start().await;

        let params_json = json!({
            "resource_names": [
                "resource_history_bytes",
                "resource_new_accounts",
                "resource_market_bytes",
                "resource_state_bytes",
                "resource_execution_time"
            ],
            "resource_params": {
                "resource_history_bytes": {
                    "price_curve_params": { "coeff_a": "1000000000000", "coeff_b": "100000", "shift": 8 },
                    "resource_dynamics_params": {
                        "resource_unit": 1,
                        "budget_per_time_unit": 40000,
                        "pool_eq": 1,
                        "max_pool_size": 1,
                        "decay_params": { "decay_per_time_unit": 1, "decay_per_time_unit_denom_shift": 1 },
                        "min_decay": 0
                    }
                },
                "resource_new_accounts": {
                    "price_curve_params": { "coeff_a": "1000000000000", "coeff_b": "100000", "shift": 8 },
                    "resource_dynamics_params": {
                        "resource_unit": 1,
                        "budget_per_time_unit": 1000,
                        "pool_eq": 1,
                        "max_pool_size": 1,
                        "decay_params": { "decay_per_time_unit": 1, "decay_per_time_unit_denom_shift": 1 },
                        "min_decay": 0
                    }
                },
                "resource_market_bytes": {
                    "price_curve_params": { "coeff_a": "1000000000000", "coeff_b": "100000", "shift": 8 },
                    "resource_dynamics_params": {
                        "resource_unit": 1,
                        "budget_per_time_unit": 10000,
                        "pool_eq": 1,
                        "max_pool_size": 1,
                        "decay_params": { "decay_per_time_unit": 1, "decay_per_time_unit_denom_shift": 1 },
                        "min_decay": 0
                    }
                },
                "resource_state_bytes": {
                    "price_curve_params": { "coeff_a": "1000000000000", "coeff_b": "100000", "shift": 8 },
                    "resource_dynamics_params": {
                        "resource_unit": 1,
                        "budget_per_time_unit": 20000,
                        "pool_eq": 1,
                        "max_pool_size": 1,
                        "decay_params": { "decay_per_time_unit": 1, "decay_per_time_unit_denom_shift": 1 },
                        "min_decay": 0
                    }
                },
                "resource_execution_time": {
                    "price_curve_params": { "coeff_a": "1000000000000", "coeff_b": "100000", "shift": 8 },
                    "resource_dynamics_params": {
                        "resource_unit": 1,
                        "budget_per_time_unit": 20000,
                        "pool_eq": 1,
                        "max_pool_size": 1,
                        "decay_params": { "decay_per_time_unit": 1, "decay_per_time_unit_denom_shift": 1 },
                        "min_decay": 0
                    }
                }
            },
            "size_info": {
                "resource_execution_time": {
                    "transaction_time": 10,
                    "verify_authority_time": 5,
                    "transfer_time": 20
                },
                "resource_state_bytes": {
                    "transaction_base_size": 7
                }
            }
        });

        let pool_json = json!({
            "resource_pool": {
                "resource_history_bytes": { "pool": 1000000, "fill_level": 10000 },
                "resource_new_accounts": { "pool": 1000000, "fill_level": 10000 },
                "resource_market_bytes": { "pool": 1000000, "fill_level": 10000 },
                "resource_state_bytes": { "pool": 1000000, "fill_level": 10000 },
                "resource_execution_time": { "pool": 1000000, "fill_level": 10000 }
            }
        });

        let stats_json = json!({
            "rc_stats": {
                "regen": 5000000,
                "share": [4000, 10000, 1000, 3000, 2000]
            }
        });

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["rc_api", "get_resource_params", {}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": params_json
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["rc_api", "get_resource_pool", {}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": pool_json
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["rc_api", "get_rc_stats", {}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": stats_json
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
        let api = RcApi::new(inner);

        let op = Operation::Transfer(TransferOperation {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: Asset::from_string("1.000 HIVE").expect("valid asset"),
            memo: "memo".to_string(),
        });

        let params = serde_json::from_value(params_json).expect("params parse");
        let pool = serde_json::from_value(pool_json).expect("pool parse");
        let stats: RcStats =
            serde_json::from_value(stats_json["rc_stats"].clone()).expect("stats parse");
        let shares = super::share_map_from_stats(&params, &stats);
        let expected = super::calculate_cost_from_state(
            std::slice::from_ref(&op),
            &params,
            &pool,
            stats.regen,
            &shares,
        )
        .expect("cost should compute");

        let actual = api
            .calculate_cost(&[op])
            .await
            .expect("calculate_cost should succeed");

        assert_eq!(actual, expected);
        assert!(actual > 0);
    }
}
