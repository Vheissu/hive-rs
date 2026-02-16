use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use crate::client::ClientInner;
use crate::crypto::{sign_transaction, PrivateKey};
use crate::error::{HiveError, Result};
use crate::serialization::generate_trx_id;
use crate::serialization::types::{format_hive_time, parse_hive_time};
use crate::types::{
    AccountCreateOperation, AccountCreateWithDelegationOperation, AccountUpdate2Operation,
    AccountUpdateOperation, AccountWitnessProxyOperation, AccountWitnessVoteOperation,
    CancelTransferFromSavingsOperation, ChangeRecoveryAccountOperation, ClaimAccountOperation,
    ClaimRewardBalanceOperation, CollateralizedConvertOperation, CommentOperation,
    CommentOptionsOperation, ConvertOperation, CreateClaimedAccountOperation,
    CreateProposalOperation, CustomBinaryOperation, CustomJsonOperation, CustomOperation,
    DeclineVotingRightsOperation, DelegateVestingSharesOperation, DeleteCommentOperation,
    DynamicGlobalProperties, EscrowApproveOperation, EscrowDisputeOperation,
    EscrowReleaseOperation, EscrowTransferOperation, FeedPublishOperation,
    LimitOrderCancelOperation, LimitOrderCreate2Operation, LimitOrderCreateOperation, Operation,
    RecoverAccountOperation, RecurrentTransferOperation, RemoveProposalOperation,
    ReportOverProductionOperation, RequestAccountRecoveryOperation, ResetAccountOperation,
    SetResetAccountOperation, SetWithdrawVestingRouteOperation, SignedTransaction, Transaction,
    TransactionConfirmation, TransferFromSavingsOperation, TransferOperation,
    TransferToSavingsOperation, TransferToVestingOperation, UpdateProposalOperation,
    UpdateProposalVotesOperation, VoteOperation, WithdrawVestingOperation, WitnessProps,
    WitnessUpdateOperation,
};
use crate::utils::build_witness_update_op;

#[derive(Debug, Clone)]
pub struct BroadcastApi {
    client: Arc<ClientInner>,
}

impl BroadcastApi {
    pub(crate) fn new(client: Arc<ClientInner>) -> Self {
        Self { client }
    }

    pub async fn create_transaction(
        &self,
        operations: Vec<Operation>,
        expiration: Option<Duration>,
    ) -> Result<Transaction> {
        let props: DynamicGlobalProperties = self
            .client
            .call("condenser_api", "get_dynamic_global_properties", json!([]))
            .await?;

        let ref_block_num = props.head_block_number & 0xFFFF;
        let block_id = hex::decode(&props.head_block_id).map_err(|err| {
            HiveError::Serialization(format!(
                "invalid head_block_id '{}': {err}",
                props.head_block_id
            ))
        })?;
        if block_id.len() < 8 {
            return Err(HiveError::Serialization(
                "head_block_id is too short to derive ref_block_prefix".to_string(),
            ));
        }
        let ref_block_prefix =
            u32::from_le_bytes(block_id[4..8].try_into().map_err(|_| {
                HiveError::Serialization("invalid ref block prefix bytes".to_string())
            })?);

        let expiration_time = expiration.unwrap_or(Duration::from_secs(60));
        let expiration_time = parse_hive_time(&props.time)?
            + chrono::Duration::from_std(expiration_time).map_err(|err| {
                HiveError::Serialization(format!("invalid expiration duration: {err}"))
            })?;

        Ok(Transaction {
            ref_block_num: ref_block_num as u16,
            ref_block_prefix,
            expiration: format_hive_time(expiration_time),
            operations,
            extensions: vec![],
        })
    }

    pub fn sign_transaction(
        &self,
        transaction: &Transaction,
        keys: &[&PrivateKey],
    ) -> Result<SignedTransaction> {
        sign_transaction(transaction, keys, &self.client.options().chain_id)
    }

    pub async fn send(&self, transaction: SignedTransaction) -> Result<TransactionConfirmation> {
        match self
            .client
            .call(
                "condenser_api",
                "broadcast_transaction_synchronous",
                json!([transaction.clone()]),
            )
            .await
        {
            Ok(confirmation) => Ok(confirmation),
            Err(err) if should_fallback_to_async_broadcast(&err) => {
                self.send_async_with_confirmation(transaction).await
            }
            Err(err) => Err(err),
        }
    }

    pub async fn send_operations(
        &self,
        operations: Vec<Operation>,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        let tx = self.create_transaction(operations, None).await?;
        let signed = self.sign_transaction(&tx, &[key])?;
        self.send(signed).await
    }

    pub async fn comment_with_options(
        &self,
        comment: CommentOperation,
        options: CommentOptionsOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(
            vec![
                Operation::Comment(comment),
                Operation::CommentOptions(options),
            ],
            key,
        )
        .await
    }

    pub async fn witness_set_properties(
        &self,
        owner: &str,
        props: WitnessProps,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        let op = build_witness_update_op(owner, props)?;
        self.send_operations(vec![Operation::WitnessSetProperties(op)], key)
            .await
    }

    pub async fn vote(
        &self,
        params: VoteOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::Vote(params)], key)
            .await
    }

    pub async fn comment(
        &self,
        params: CommentOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::Comment(params)], key)
            .await
    }

    pub async fn transfer(
        &self,
        params: TransferOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::Transfer(params)], key)
            .await
    }

    pub async fn transfer_to_vesting(
        &self,
        params: TransferToVestingOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::TransferToVesting(params)], key)
            .await
    }

    pub async fn withdraw_vesting(
        &self,
        params: WithdrawVestingOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::WithdrawVesting(params)], key)
            .await
    }

    pub async fn limit_order_create(
        &self,
        params: LimitOrderCreateOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::LimitOrderCreate(params)], key)
            .await
    }

    pub async fn limit_order_cancel(
        &self,
        params: LimitOrderCancelOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::LimitOrderCancel(params)], key)
            .await
    }

    pub async fn feed_publish(
        &self,
        params: FeedPublishOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::FeedPublish(params)], key)
            .await
    }

    pub async fn convert(
        &self,
        params: ConvertOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::Convert(params)], key)
            .await
    }

    pub async fn account_create(
        &self,
        params: AccountCreateOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::AccountCreate(params)], key)
            .await
    }

    pub async fn account_update(
        &self,
        params: AccountUpdateOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::AccountUpdate(params)], key)
            .await
    }

    pub async fn witness_update(
        &self,
        params: WitnessUpdateOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::WitnessUpdate(params)], key)
            .await
    }

    pub async fn account_witness_vote(
        &self,
        params: AccountWitnessVoteOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::AccountWitnessVote(params)], key)
            .await
    }

    pub async fn account_witness_proxy(
        &self,
        params: AccountWitnessProxyOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::AccountWitnessProxy(params)], key)
            .await
    }

    pub async fn custom(
        &self,
        params: CustomOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::Custom(params)], key)
            .await
    }

    pub async fn report_over_production(
        &self,
        params: ReportOverProductionOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::ReportOverProduction(params)], key)
            .await
    }

    pub async fn delete_comment(
        &self,
        params: DeleteCommentOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::DeleteComment(params)], key)
            .await
    }

    pub async fn custom_json(
        &self,
        params: CustomJsonOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::CustomJson(params)], key)
            .await
    }

    pub async fn comment_options(
        &self,
        params: CommentOptionsOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::CommentOptions(params)], key)
            .await
    }

    pub async fn set_withdraw_vesting_route(
        &self,
        params: SetWithdrawVestingRouteOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::SetWithdrawVestingRoute(params)], key)
            .await
    }

    pub async fn limit_order_create2(
        &self,
        params: LimitOrderCreate2Operation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::LimitOrderCreate2(params)], key)
            .await
    }

    pub async fn claim_account(
        &self,
        params: ClaimAccountOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::ClaimAccount(params)], key)
            .await
    }

    pub async fn create_claimed_account(
        &self,
        params: CreateClaimedAccountOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::CreateClaimedAccount(params)], key)
            .await
    }

    pub async fn request_account_recovery(
        &self,
        params: RequestAccountRecoveryOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::RequestAccountRecovery(params)], key)
            .await
    }

    pub async fn recover_account(
        &self,
        params: RecoverAccountOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::RecoverAccount(params)], key)
            .await
    }

    pub async fn change_recovery_account(
        &self,
        params: ChangeRecoveryAccountOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::ChangeRecoveryAccount(params)], key)
            .await
    }

    pub async fn escrow_transfer(
        &self,
        params: EscrowTransferOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::EscrowTransfer(params)], key)
            .await
    }

    pub async fn escrow_dispute(
        &self,
        params: EscrowDisputeOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::EscrowDispute(params)], key)
            .await
    }

    pub async fn escrow_release(
        &self,
        params: EscrowReleaseOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::EscrowRelease(params)], key)
            .await
    }

    pub async fn escrow_approve(
        &self,
        params: EscrowApproveOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::EscrowApprove(params)], key)
            .await
    }

    pub async fn transfer_to_savings(
        &self,
        params: TransferToSavingsOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::TransferToSavings(params)], key)
            .await
    }

    pub async fn transfer_from_savings(
        &self,
        params: TransferFromSavingsOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::TransferFromSavings(params)], key)
            .await
    }

    pub async fn cancel_transfer_from_savings(
        &self,
        params: CancelTransferFromSavingsOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::CancelTransferFromSavings(params)], key)
            .await
    }

    pub async fn custom_binary(
        &self,
        params: CustomBinaryOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::CustomBinary(params)], key)
            .await
    }

    pub async fn decline_voting_rights(
        &self,
        params: DeclineVotingRightsOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::DeclineVotingRights(params)], key)
            .await
    }

    pub async fn reset_account(
        &self,
        params: ResetAccountOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::ResetAccount(params)], key)
            .await
    }

    pub async fn set_reset_account(
        &self,
        params: SetResetAccountOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::SetResetAccount(params)], key)
            .await
    }

    pub async fn claim_reward_balance(
        &self,
        params: ClaimRewardBalanceOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::ClaimRewardBalance(params)], key)
            .await
    }

    pub async fn delegate_vesting_shares(
        &self,
        params: DelegateVestingSharesOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::DelegateVestingShares(params)], key)
            .await
    }

    pub async fn account_create_with_delegation(
        &self,
        params: AccountCreateWithDelegationOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::AccountCreateWithDelegation(params)], key)
            .await
    }

    pub async fn account_update2(
        &self,
        params: AccountUpdate2Operation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::AccountUpdate2(params)], key)
            .await
    }

    pub async fn create_proposal(
        &self,
        params: CreateProposalOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::CreateProposal(params)], key)
            .await
    }

    pub async fn update_proposal_votes(
        &self,
        params: UpdateProposalVotesOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::UpdateProposalVotes(params)], key)
            .await
    }

    pub async fn remove_proposal(
        &self,
        params: RemoveProposalOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::RemoveProposal(params)], key)
            .await
    }

    pub async fn update_proposal(
        &self,
        params: UpdateProposalOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::UpdateProposal(params)], key)
            .await
    }

    pub async fn collateralized_convert(
        &self,
        params: CollateralizedConvertOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::CollateralizedConvert(params)], key)
            .await
    }

    pub async fn recurrent_transfer(
        &self,
        params: RecurrentTransferOperation,
        key: &PrivateKey,
    ) -> Result<TransactionConfirmation> {
        self.send_operations(vec![Operation::RecurrentTransfer(params)], key)
            .await
    }

    async fn send_async_with_confirmation(
        &self,
        transaction: SignedTransaction,
    ) -> Result<TransactionConfirmation> {
        let tx_id = signed_transaction_id(&transaction)?;

        let _: Value = self
            .client
            .call(
                "condenser_api",
                "broadcast_transaction",
                json!([transaction]),
            )
            .await?;

        for _ in 0..15 {
            match self
                .client
                .call::<Value>("condenser_api", "get_transaction", json!([tx_id.clone()]))
                .await
            {
                Ok(found) => return Ok(confirmation_from_condenser_transaction(&tx_id, &found)),
                Err(err) if is_transient_lookup_error(&err) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }

        // The async broadcast call succeeded, but the tx was not yet visible in the lookup window.
        Ok(TransactionConfirmation {
            id: tx_id,
            block_num: 0,
            trx_num: 0,
            expired: false,
        })
    }
}

fn should_fallback_to_async_broadcast(error: &HiveError) -> bool {
    match error {
        HiveError::Transport(_) | HiveError::Timeout | HiveError::AllNodesFailed => true,
        HiveError::Serialization(_) => true,
        HiveError::Rpc { message, .. } => {
            let message = message.to_ascii_lowercase();
            message.contains("could not find method") || message.contains("could not find api")
        }
        _ => false,
    }
}

fn signed_transaction_id(transaction: &SignedTransaction) -> Result<String> {
    let unsigned = Transaction {
        ref_block_num: transaction.ref_block_num,
        ref_block_prefix: transaction.ref_block_prefix,
        expiration: transaction.expiration.clone(),
        operations: transaction.operations.clone(),
        extensions: transaction.extensions.clone(),
    };

    generate_trx_id(&unsigned)
}

fn confirmation_from_condenser_transaction(
    tx_id: &str,
    transaction: &Value,
) -> TransactionConfirmation {
    let block_num = transaction
        .get("block_num")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
    let trx_num = transaction
        .get("transaction_num")
        .or_else(|| transaction.get("trx_num"))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);

    TransactionConfirmation {
        id: tx_id.to_string(),
        block_num,
        trx_num,
        expired: false,
    }
}

fn is_transient_lookup_error(error: &HiveError) -> bool {
    match error {
        HiveError::Transport(_) | HiveError::Timeout | HiveError::AllNodesFailed => true,
        HiveError::Rpc { message, .. } => {
            let message = message.to_ascii_lowercase();
            message.contains("unknown transaction")
                || message.contains("unable to find transaction")
                || message.contains("missing transaction")
                || message.contains("could not find method")
                || message.contains("could not find api")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::api::BroadcastApi;
    use crate::client::{ClientInner, ClientOptions};
    use crate::crypto::PrivateKey;
    use crate::transport::{BackoffStrategy, FailoverTransport};
    use crate::types::{Asset, Operation, SignedTransaction, TransferOperation};

    #[tokio::test]
    async fn send_operations_builds_signs_and_broadcasts() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["condenser_api", "get_dynamic_global_properties", []]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "head_block_number": 42,
                    "head_block_id": "0000002a11223344556677889900aabbccddeeff00112233445566778899aabb",
                    "time": "2024-01-01T00:00:00",
                    "last_irreversible_block_num": 41
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["condenser_api", "broadcast_transaction_synchronous"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "id": "abc",
                    "block_num": 42,
                    "trx_num": 1,
                    "expired": false
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
        let broadcast = BroadcastApi::new(inner);

        let key = PrivateKey::from_wif("5KG4sr3rMH1QuduYj79p36h7PrEeZakHEPjB9NkLWqgw19DDieL")
            .expect("valid private key");

        let result = broadcast
            .send_operations(
                vec![Operation::Transfer(TransferOperation {
                    from: "foo".to_string(),
                    to: "bar".to_string(),
                    amount: Asset::from_string("1.000 HIVE").expect("asset should parse"),
                    memo: "test".to_string(),
                })],
                &key,
            )
            .await
            .expect("operation should broadcast");

        assert_eq!(result.block_num, 42);
        assert!(!result.expired);
    }

    #[tokio::test]
    async fn send_falls_back_to_async_broadcast_when_sync_endpoint_fails() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["condenser_api", "broadcast_transaction_synchronous"]
            })))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["condenser_api", "broadcast_transaction"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {}
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "call",
                "params": ["condenser_api", "get_transaction"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "block_num": 42,
                    "transaction_num": 7
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
        let broadcast = BroadcastApi::new(inner);

        let tx = SignedTransaction {
            ref_block_num: 1,
            ref_block_prefix: 2,
            expiration: "2024-01-01T00:00:00".to_string(),
            operations: vec![],
            extensions: vec![],
            signatures: vec!["1f00".to_string()],
        };

        let result = broadcast.send(tx).await.expect("fallback should succeed");
        assert_eq!(result.block_num, 42);
        assert_eq!(result.trx_num, 7);
        assert!(!result.id.is_empty());
    }
}
