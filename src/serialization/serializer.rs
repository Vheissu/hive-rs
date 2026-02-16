use crate::crypto::utils::sha256;
use crate::error::{HiveError, Result};
use crate::serialization::types::{
    write_array, write_asset, write_authority, write_bool, write_chain_properties, write_date,
    write_flat_map, write_i16, write_i64, write_optional, write_price, write_public_key,
    write_string, write_u16, write_u32, write_u64, write_variable_binary, write_varint32,
    write_void_array,
};
use crate::types::{
    AccountCreateOperation, AccountCreateWithDelegationOperation, AccountUpdate2Operation,
    AccountUpdateOperation, AccountWitnessProxyOperation, AccountWitnessVoteOperation,
    CancelTransferFromSavingsOperation, ChainId, ChangeRecoveryAccountOperation,
    ClaimAccountOperation, ClaimRewardBalanceOperation, CollateralizedConvertOperation,
    CommentOperation, CommentOptionsExtension, CommentOptionsOperation, ConvertOperation,
    CreateClaimedAccountOperation, CreateProposalOperation, CustomBinaryOperation,
    CustomJsonOperation, CustomOperation, DeclineVotingRightsOperation,
    DelegateVestingSharesOperation, DeleteCommentOperation, EscrowApproveOperation,
    EscrowDisputeOperation, EscrowReleaseOperation, EscrowTransferOperation, FeedPublishOperation,
    LimitOrderCancelOperation, LimitOrderCreate2Operation, LimitOrderCreateOperation, Operation,
    Pow2Operation, PowOperation, RecoverAccountOperation, RecurrentTransferOperation,
    RemoveProposalOperation, ReportOverProductionOperation, RequestAccountRecoveryOperation,
    ResetAccountOperation, SetResetAccountOperation, SetWithdrawVestingRouteOperation,
    SignedBlockHeader, Transaction, TransferFromSavingsOperation, TransferOperation,
    TransferToSavingsOperation, TransferToVestingOperation, UpdateProposalExtension,
    UpdateProposalOperation, UpdateProposalVotesOperation, VoteOperation, WithdrawVestingOperation,
    WitnessSetPropertiesOperation, WitnessUpdateOperation,
};

pub trait HiveSerialize {
    fn hive_serialize(&self, buf: &mut Vec<u8>) -> Result<()>;
}

impl HiveSerialize for Operation {
    fn hive_serialize(&self, buf: &mut Vec<u8>) -> Result<()> {
        write_varint32(buf, self.id() as u32);
        match self {
            Operation::Vote(op) => serialize_vote(buf, op),
            Operation::Comment(op) => serialize_comment(buf, op),
            Operation::Transfer(op) => serialize_transfer(buf, op),
            Operation::TransferToVesting(op) => serialize_transfer_to_vesting(buf, op),
            Operation::WithdrawVesting(op) => serialize_withdraw_vesting(buf, op),
            Operation::LimitOrderCreate(op) => serialize_limit_order_create(buf, op),
            Operation::LimitOrderCancel(op) => serialize_limit_order_cancel(buf, op),
            Operation::FeedPublish(op) => serialize_feed_publish(buf, op),
            Operation::Convert(op) => serialize_convert(buf, op),
            Operation::AccountCreate(op) => serialize_account_create(buf, op),
            Operation::AccountUpdate(op) => serialize_account_update(buf, op),
            Operation::WitnessUpdate(op) => serialize_witness_update(buf, op),
            Operation::AccountWitnessVote(op) => serialize_account_witness_vote(buf, op),
            Operation::AccountWitnessProxy(op) => serialize_account_witness_proxy(buf, op),
            Operation::Pow(op) => serialize_pow(buf, op),
            Operation::Custom(op) => serialize_custom(buf, op),
            Operation::ReportOverProduction(op) => serialize_report_over_production(buf, op),
            Operation::DeleteComment(op) => serialize_delete_comment(buf, op),
            Operation::CustomJson(op) => serialize_custom_json(buf, op),
            Operation::CommentOptions(op) => serialize_comment_options(buf, op),
            Operation::SetWithdrawVestingRoute(op) => serialize_set_withdraw_vesting_route(buf, op),
            Operation::LimitOrderCreate2(op) => serialize_limit_order_create2(buf, op),
            Operation::ClaimAccount(op) => serialize_claim_account(buf, op),
            Operation::CreateClaimedAccount(op) => serialize_create_claimed_account(buf, op),
            Operation::RequestAccountRecovery(op) => serialize_request_account_recovery(buf, op),
            Operation::RecoverAccount(op) => serialize_recover_account(buf, op),
            Operation::ChangeRecoveryAccount(op) => serialize_change_recovery_account(buf, op),
            Operation::EscrowTransfer(op) => serialize_escrow_transfer(buf, op),
            Operation::EscrowDispute(op) => serialize_escrow_dispute(buf, op),
            Operation::EscrowRelease(op) => serialize_escrow_release(buf, op),
            Operation::Pow2(op) => serialize_pow2(buf, op),
            Operation::EscrowApprove(op) => serialize_escrow_approve(buf, op),
            Operation::TransferToSavings(op) => serialize_transfer_to_savings(buf, op),
            Operation::TransferFromSavings(op) => serialize_transfer_from_savings(buf, op),
            Operation::CancelTransferFromSavings(op) => {
                serialize_cancel_transfer_from_savings(buf, op)
            }
            Operation::CustomBinary(op) => serialize_custom_binary(buf, op),
            Operation::DeclineVotingRights(op) => serialize_decline_voting_rights(buf, op),
            Operation::ResetAccount(op) => serialize_reset_account(buf, op),
            Operation::SetResetAccount(op) => serialize_set_reset_account(buf, op),
            Operation::ClaimRewardBalance(op) => serialize_claim_reward_balance(buf, op),
            Operation::DelegateVestingShares(op) => serialize_delegate_vesting_shares(buf, op),
            Operation::AccountCreateWithDelegation(op) => {
                serialize_account_create_with_delegation(buf, op)
            }
            Operation::WitnessSetProperties(op) => serialize_witness_set_properties(buf, op),
            Operation::AccountUpdate2(op) => serialize_account_update2(buf, op),
            Operation::CreateProposal(op) => serialize_create_proposal(buf, op),
            Operation::UpdateProposalVotes(op) => serialize_update_proposal_votes(buf, op),
            Operation::RemoveProposal(op) => serialize_remove_proposal(buf, op),
            Operation::UpdateProposal(op) => serialize_update_proposal(buf, op),
            Operation::CollateralizedConvert(op) => serialize_collateralized_convert(buf, op),
            Operation::RecurrentTransfer(op) => serialize_recurrent_transfer(buf, op),
        }
    }
}

impl HiveSerialize for Transaction {
    fn hive_serialize(&self, buf: &mut Vec<u8>) -> Result<()> {
        write_u16(buf, self.ref_block_num);
        write_u32(buf, self.ref_block_prefix);
        write_date(buf, &self.expiration)?;
        write_array(buf, &self.operations, |b, op| op.hive_serialize(b))?;
        write_array(buf, &self.extensions, |b, ext| {
            write_string(b, ext);
            Ok(())
        })?;
        Ok(())
    }
}

pub fn serialize_transaction(transaction: &Transaction) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    transaction.hive_serialize(&mut buf)?;
    Ok(buf)
}

pub fn transaction_digest(transaction: &Transaction, chain_id: &ChainId) -> Result<[u8; 32]> {
    let tx_bytes = serialize_transaction(transaction)?;
    let mut to_hash = Vec::with_capacity(chain_id.bytes.len() + tx_bytes.len());
    to_hash.extend_from_slice(&chain_id.bytes);
    to_hash.extend_from_slice(&tx_bytes);
    Ok(sha256(&to_hash))
}

pub fn generate_trx_id(transaction: &Transaction) -> Result<String> {
    let tx_bytes = serialize_transaction(transaction)?;
    let hash = sha256(&tx_bytes);
    Ok(hex::encode(hash)[..40].to_string())
}

fn write_void_extensions(buf: &mut Vec<u8>, extensions: &[()]) -> Result<()> {
    if !extensions.is_empty() {
        return Err(HiveError::Serialization(
            "void extensions must be empty".to_string(),
        ));
    }
    write_void_array(buf);
    Ok(())
}

fn write_fixed_binary_hex(buf: &mut Vec<u8>, hex_value: &str, expected_len: usize) -> Result<()> {
    let bytes = hex::decode(hex_value).map_err(|err| {
        HiveError::Serialization(format!("invalid hex field '{hex_value}': {err}"))
    })?;
    if bytes.len() != expected_len {
        return Err(HiveError::Serialization(format!(
            "expected {expected_len} bytes, got {}",
            bytes.len()
        )));
    }
    buf.extend_from_slice(&bytes);
    Ok(())
}

fn write_signed_block_header(buf: &mut Vec<u8>, header: &SignedBlockHeader) -> Result<()> {
    write_fixed_binary_hex(buf, &header.header.previous, 20)?;
    write_date(buf, &header.header.timestamp)?;
    write_string(buf, &header.header.witness);
    write_fixed_binary_hex(buf, &header.header.transaction_merkle_root, 20)?;
    if !header.header.extensions.is_empty() {
        return Err(HiveError::Serialization(
            "signed block header extensions are expected to be empty".to_string(),
        ));
    }
    write_void_array(buf);
    write_fixed_binary_hex(buf, &header.witness_signature, 65)
}

fn serialize_vote(buf: &mut Vec<u8>, op: &VoteOperation) -> Result<()> {
    write_string(buf, &op.voter);
    write_string(buf, &op.author);
    write_string(buf, &op.permlink);
    write_i16(buf, op.weight);
    Ok(())
}

fn serialize_comment(buf: &mut Vec<u8>, op: &CommentOperation) -> Result<()> {
    write_string(buf, &op.parent_author);
    write_string(buf, &op.parent_permlink);
    write_string(buf, &op.author);
    write_string(buf, &op.permlink);
    write_string(buf, &op.title);
    write_string(buf, &op.body);
    write_string(buf, &op.json_metadata);
    Ok(())
}

fn serialize_transfer(buf: &mut Vec<u8>, op: &TransferOperation) -> Result<()> {
    write_string(buf, &op.from);
    write_string(buf, &op.to);
    write_asset(buf, &op.amount)?;
    write_string(buf, &op.memo);
    Ok(())
}

fn serialize_transfer_to_vesting(buf: &mut Vec<u8>, op: &TransferToVestingOperation) -> Result<()> {
    write_string(buf, &op.from);
    write_string(buf, &op.to);
    write_asset(buf, &op.amount)
}

fn serialize_withdraw_vesting(buf: &mut Vec<u8>, op: &WithdrawVestingOperation) -> Result<()> {
    write_string(buf, &op.account);
    write_asset(buf, &op.vesting_shares)
}

fn serialize_limit_order_create(buf: &mut Vec<u8>, op: &LimitOrderCreateOperation) -> Result<()> {
    write_string(buf, &op.owner);
    write_u32(buf, op.orderid);
    write_asset(buf, &op.amount_to_sell)?;
    write_asset(buf, &op.min_to_receive)?;
    write_bool(buf, op.fill_or_kill);
    write_date(buf, &op.expiration)
}

fn serialize_limit_order_cancel(buf: &mut Vec<u8>, op: &LimitOrderCancelOperation) -> Result<()> {
    write_string(buf, &op.owner);
    write_u32(buf, op.orderid);
    Ok(())
}

fn serialize_feed_publish(buf: &mut Vec<u8>, op: &FeedPublishOperation) -> Result<()> {
    write_string(buf, &op.publisher);
    write_price(buf, &op.exchange_rate)
}

fn serialize_convert(buf: &mut Vec<u8>, op: &ConvertOperation) -> Result<()> {
    write_string(buf, &op.owner);
    write_u32(buf, op.requestid);
    write_asset(buf, &op.amount)
}

fn serialize_account_create(buf: &mut Vec<u8>, op: &AccountCreateOperation) -> Result<()> {
    write_asset(buf, &op.fee)?;
    write_string(buf, &op.creator);
    write_string(buf, &op.new_account_name);
    write_authority(buf, &op.owner)?;
    write_authority(buf, &op.active)?;
    write_authority(buf, &op.posting)?;
    write_public_key(buf, &op.memo_key)?;
    write_string(buf, &op.json_metadata);
    Ok(())
}

fn serialize_account_update(buf: &mut Vec<u8>, op: &AccountUpdateOperation) -> Result<()> {
    write_string(buf, &op.account);
    write_optional(buf, op.owner.as_ref(), write_authority)?;
    write_optional(buf, op.active.as_ref(), write_authority)?;
    write_optional(buf, op.posting.as_ref(), write_authority)?;
    write_public_key(buf, &op.memo_key)?;
    write_string(buf, &op.json_metadata);
    Ok(())
}

fn serialize_witness_update(buf: &mut Vec<u8>, op: &WitnessUpdateOperation) -> Result<()> {
    write_string(buf, &op.owner);
    write_string(buf, &op.url);
    write_public_key(buf, &op.block_signing_key)?;
    write_chain_properties(buf, &op.props)?;
    write_asset(buf, &op.fee)
}

fn serialize_account_witness_vote(
    buf: &mut Vec<u8>,
    op: &AccountWitnessVoteOperation,
) -> Result<()> {
    write_string(buf, &op.account);
    write_string(buf, &op.witness);
    write_bool(buf, op.approve);
    Ok(())
}

fn serialize_account_witness_proxy(
    buf: &mut Vec<u8>,
    op: &AccountWitnessProxyOperation,
) -> Result<()> {
    write_string(buf, &op.account);
    write_string(buf, &op.proxy);
    Ok(())
}

fn serialize_pow(_buf: &mut Vec<u8>, _op: &PowOperation) -> Result<()> {
    Err(HiveError::Serialization(
        "pow operation serialization is unsupported".to_string(),
    ))
}

fn serialize_custom(buf: &mut Vec<u8>, op: &CustomOperation) -> Result<()> {
    write_array(buf, &op.required_auths, |b, auth| {
        write_string(b, auth);
        Ok(())
    })?;
    write_u16(buf, op.id);
    write_variable_binary(buf, &op.data);
    Ok(())
}

fn serialize_report_over_production(
    buf: &mut Vec<u8>,
    op: &ReportOverProductionOperation,
) -> Result<()> {
    write_string(buf, &op.reporter);
    write_signed_block_header(buf, &op.first_block)?;
    write_signed_block_header(buf, &op.second_block)?;
    Ok(())
}

fn serialize_delete_comment(buf: &mut Vec<u8>, op: &DeleteCommentOperation) -> Result<()> {
    write_string(buf, &op.author);
    write_string(buf, &op.permlink);
    Ok(())
}

fn serialize_custom_json(buf: &mut Vec<u8>, op: &CustomJsonOperation) -> Result<()> {
    write_array(buf, &op.required_auths, |b, auth| {
        write_string(b, auth);
        Ok(())
    })?;
    write_array(buf, &op.required_posting_auths, |b, auth| {
        write_string(b, auth);
        Ok(())
    })?;
    write_string(buf, &op.id);
    write_string(buf, &op.json);
    Ok(())
}

fn serialize_comment_options(buf: &mut Vec<u8>, op: &CommentOptionsOperation) -> Result<()> {
    write_string(buf, &op.author);
    write_string(buf, &op.permlink);
    write_asset(buf, &op.max_accepted_payout)?;
    write_u16(buf, op.percent_hbd);
    write_bool(buf, op.allow_votes);
    write_bool(buf, op.allow_curation_rewards);
    write_array(buf, &op.extensions, |b, ext| match ext {
        CommentOptionsExtension::Beneficiaries { beneficiaries } => {
            write_varint32(b, 0);
            write_array(b, beneficiaries, |bb, route| {
                write_string(bb, &route.account);
                write_u16(bb, route.weight);
                Ok(())
            })
        }
    })?;
    Ok(())
}

fn serialize_set_withdraw_vesting_route(
    buf: &mut Vec<u8>,
    op: &SetWithdrawVestingRouteOperation,
) -> Result<()> {
    write_string(buf, &op.from_account);
    write_string(buf, &op.to_account);
    write_u16(buf, op.percent);
    write_bool(buf, op.auto_vest);
    Ok(())
}

fn serialize_limit_order_create2(buf: &mut Vec<u8>, op: &LimitOrderCreate2Operation) -> Result<()> {
    write_string(buf, &op.owner);
    write_u32(buf, op.orderid);
    write_asset(buf, &op.amount_to_sell)?;
    write_price(buf, &op.exchange_rate)?;
    write_bool(buf, op.fill_or_kill);
    write_date(buf, &op.expiration)
}

fn serialize_claim_account(buf: &mut Vec<u8>, op: &ClaimAccountOperation) -> Result<()> {
    write_string(buf, &op.creator);
    write_asset(buf, &op.fee)?;
    write_void_extensions(buf, &op.extensions)
}

fn serialize_create_claimed_account(
    buf: &mut Vec<u8>,
    op: &CreateClaimedAccountOperation,
) -> Result<()> {
    write_string(buf, &op.creator);
    write_string(buf, &op.new_account_name);
    write_authority(buf, &op.owner)?;
    write_authority(buf, &op.active)?;
    write_authority(buf, &op.posting)?;
    write_public_key(buf, &op.memo_key)?;
    write_string(buf, &op.json_metadata);
    write_void_extensions(buf, &op.extensions)
}

fn serialize_request_account_recovery(
    buf: &mut Vec<u8>,
    op: &RequestAccountRecoveryOperation,
) -> Result<()> {
    write_string(buf, &op.recovery_account);
    write_string(buf, &op.account_to_recover);
    write_authority(buf, &op.new_owner_authority)?;
    write_void_extensions(buf, &op.extensions)
}

fn serialize_recover_account(buf: &mut Vec<u8>, op: &RecoverAccountOperation) -> Result<()> {
    write_string(buf, &op.account_to_recover);
    write_authority(buf, &op.new_owner_authority)?;
    write_authority(buf, &op.recent_owner_authority)?;
    write_void_extensions(buf, &op.extensions)
}

fn serialize_change_recovery_account(
    buf: &mut Vec<u8>,
    op: &ChangeRecoveryAccountOperation,
) -> Result<()> {
    write_string(buf, &op.account_to_recover);
    write_string(buf, &op.new_recovery_account);
    write_void_extensions(buf, &op.extensions)
}

fn serialize_escrow_transfer(buf: &mut Vec<u8>, op: &EscrowTransferOperation) -> Result<()> {
    write_string(buf, &op.from);
    write_string(buf, &op.to);
    write_asset(buf, &op.hbd_amount)?;
    write_asset(buf, &op.hive_amount)?;
    write_u32(buf, op.escrow_id);
    write_string(buf, &op.agent);
    write_asset(buf, &op.fee)?;
    write_string(buf, &op.json_meta);
    write_date(buf, &op.ratification_deadline)?;
    write_date(buf, &op.escrow_expiration)
}

fn serialize_escrow_dispute(buf: &mut Vec<u8>, op: &EscrowDisputeOperation) -> Result<()> {
    write_string(buf, &op.from);
    write_string(buf, &op.to);
    write_string(buf, &op.agent);
    write_string(buf, &op.who);
    write_u32(buf, op.escrow_id);
    Ok(())
}

fn serialize_escrow_release(buf: &mut Vec<u8>, op: &EscrowReleaseOperation) -> Result<()> {
    write_string(buf, &op.from);
    write_string(buf, &op.to);
    write_string(buf, &op.agent);
    write_string(buf, &op.who);
    write_string(buf, &op.receiver);
    write_u32(buf, op.escrow_id);
    write_asset(buf, &op.hbd_amount)?;
    write_asset(buf, &op.hive_amount)?;
    Ok(())
}

fn serialize_pow2(_buf: &mut Vec<u8>, _op: &Pow2Operation) -> Result<()> {
    Err(HiveError::Serialization(
        "pow2 operation serialization is unsupported".to_string(),
    ))
}

fn serialize_escrow_approve(buf: &mut Vec<u8>, op: &EscrowApproveOperation) -> Result<()> {
    write_string(buf, &op.from);
    write_string(buf, &op.to);
    write_string(buf, &op.agent);
    write_string(buf, &op.who);
    write_u32(buf, op.escrow_id);
    write_bool(buf, op.approve);
    Ok(())
}

fn serialize_transfer_to_savings(buf: &mut Vec<u8>, op: &TransferToSavingsOperation) -> Result<()> {
    write_string(buf, &op.from);
    write_string(buf, &op.to);
    write_asset(buf, &op.amount)?;
    write_string(buf, &op.memo);
    Ok(())
}

fn serialize_transfer_from_savings(
    buf: &mut Vec<u8>,
    op: &TransferFromSavingsOperation,
) -> Result<()> {
    write_string(buf, &op.from);
    write_u32(buf, op.request_id);
    write_string(buf, &op.to);
    write_asset(buf, &op.amount)?;
    write_string(buf, &op.memo);
    Ok(())
}

fn serialize_cancel_transfer_from_savings(
    buf: &mut Vec<u8>,
    op: &CancelTransferFromSavingsOperation,
) -> Result<()> {
    write_string(buf, &op.from);
    write_u32(buf, op.request_id);
    Ok(())
}

fn serialize_custom_binary(buf: &mut Vec<u8>, op: &CustomBinaryOperation) -> Result<()> {
    write_array(buf, &op.required_owner_auths, |b, value| {
        write_string(b, value);
        Ok(())
    })?;
    write_array(buf, &op.required_active_auths, |b, value| {
        write_string(b, value);
        Ok(())
    })?;
    write_array(buf, &op.required_posting_auths, |b, value| {
        write_string(b, value);
        Ok(())
    })?;
    write_array(buf, &op.required_auths, write_authority)?;
    write_string(buf, &op.id);
    write_variable_binary(buf, &op.data);
    Ok(())
}

fn serialize_decline_voting_rights(
    buf: &mut Vec<u8>,
    op: &DeclineVotingRightsOperation,
) -> Result<()> {
    write_string(buf, &op.account);
    write_bool(buf, op.decline);
    Ok(())
}

fn serialize_reset_account(buf: &mut Vec<u8>, op: &ResetAccountOperation) -> Result<()> {
    write_string(buf, &op.reset_account);
    write_string(buf, &op.account_to_reset);
    write_authority(buf, &op.new_owner_authority)
}

fn serialize_set_reset_account(buf: &mut Vec<u8>, op: &SetResetAccountOperation) -> Result<()> {
    write_string(buf, &op.account);
    write_string(buf, &op.current_reset_account);
    write_string(buf, &op.reset_account);
    Ok(())
}

fn serialize_claim_reward_balance(
    buf: &mut Vec<u8>,
    op: &ClaimRewardBalanceOperation,
) -> Result<()> {
    write_string(buf, &op.account);
    write_asset(buf, &op.reward_hive)?;
    write_asset(buf, &op.reward_hbd)?;
    write_asset(buf, &op.reward_vests)
}

fn serialize_delegate_vesting_shares(
    buf: &mut Vec<u8>,
    op: &DelegateVestingSharesOperation,
) -> Result<()> {
    write_string(buf, &op.delegator);
    write_string(buf, &op.delegatee);
    write_asset(buf, &op.vesting_shares)
}

fn serialize_account_create_with_delegation(
    buf: &mut Vec<u8>,
    op: &AccountCreateWithDelegationOperation,
) -> Result<()> {
    write_asset(buf, &op.fee)?;
    write_asset(buf, &op.delegation)?;
    write_string(buf, &op.creator);
    write_string(buf, &op.new_account_name);
    write_authority(buf, &op.owner)?;
    write_authority(buf, &op.active)?;
    write_authority(buf, &op.posting)?;
    write_public_key(buf, &op.memo_key)?;
    write_string(buf, &op.json_metadata);
    write_void_extensions(buf, &op.extensions)
}

fn serialize_witness_set_properties(
    buf: &mut Vec<u8>,
    op: &WitnessSetPropertiesOperation,
) -> Result<()> {
    write_string(buf, &op.owner);
    let mut props = op.props.clone();
    props.sort_by(|a, b| a.0.cmp(&b.0));
    write_flat_map(
        buf,
        &props,
        |b, key| {
            write_string(b, key);
            Ok(())
        },
        |b, value| {
            write_variable_binary(b, value);
            Ok(())
        },
    )?;
    write_void_extensions(buf, &op.extensions)
}

fn serialize_account_update2(buf: &mut Vec<u8>, op: &AccountUpdate2Operation) -> Result<()> {
    write_string(buf, &op.account);
    write_optional(buf, op.owner.as_ref(), write_authority)?;
    write_optional(buf, op.active.as_ref(), write_authority)?;
    write_optional(buf, op.posting.as_ref(), write_authority)?;
    write_optional(buf, op.memo_key.as_ref(), |b, key| write_public_key(b, key))?;
    write_string(buf, &op.json_metadata);
    write_string(buf, &op.posting_json_metadata);
    write_void_extensions(buf, &op.extensions)
}

fn serialize_create_proposal(buf: &mut Vec<u8>, op: &CreateProposalOperation) -> Result<()> {
    write_string(buf, &op.creator);
    write_string(buf, &op.receiver);
    write_date(buf, &op.start_date)?;
    write_date(buf, &op.end_date)?;
    write_asset(buf, &op.daily_pay)?;
    write_string(buf, &op.subject);
    write_string(buf, &op.permlink);
    write_void_extensions(buf, &op.extensions)
}

fn serialize_update_proposal_votes(
    buf: &mut Vec<u8>,
    op: &UpdateProposalVotesOperation,
) -> Result<()> {
    write_string(buf, &op.voter);
    write_array(buf, &op.proposal_ids, |b, id| {
        write_i64(b, *id);
        Ok(())
    })?;
    write_bool(buf, op.approve);
    write_void_extensions(buf, &op.extensions)
}

fn serialize_remove_proposal(buf: &mut Vec<u8>, op: &RemoveProposalOperation) -> Result<()> {
    write_string(buf, &op.proposal_owner);
    write_array(buf, &op.proposal_ids, |b, id| {
        write_i64(b, *id);
        Ok(())
    })?;
    write_void_extensions(buf, &op.extensions)
}

fn serialize_update_proposal(buf: &mut Vec<u8>, op: &UpdateProposalOperation) -> Result<()> {
    write_u64(buf, op.proposal_id);
    write_string(buf, &op.creator);
    write_asset(buf, &op.daily_pay)?;
    write_string(buf, &op.subject);
    write_string(buf, &op.permlink);
    write_array(buf, &op.extensions, |b, ext| match ext {
        UpdateProposalExtension::Void => {
            write_varint32(b, 0);
            Ok(())
        }
        UpdateProposalExtension::EndDate { end_date } => {
            write_varint32(b, 1);
            write_date(b, end_date)
        }
    })?;
    Ok(())
}

fn serialize_collateralized_convert(
    buf: &mut Vec<u8>,
    op: &CollateralizedConvertOperation,
) -> Result<()> {
    write_string(buf, &op.owner);
    write_u32(buf, op.requestid);
    write_asset(buf, &op.amount)
}

fn serialize_recurrent_transfer(buf: &mut Vec<u8>, op: &RecurrentTransferOperation) -> Result<()> {
    write_string(buf, &op.from);
    write_string(buf, &op.to);
    write_asset(buf, &op.amount)?;
    write_string(buf, &op.memo);
    write_u16(buf, op.recurrence);
    write_u16(buf, op.executions);
    write_void_extensions(buf, &op.extensions)
}

#[cfg(test)]
mod tests {
    use crate::serialization::serializer::{
        generate_trx_id, serialize_transaction, transaction_digest, HiveSerialize,
    };
    use crate::types::Asset;
    use crate::types::{ChainId, Operation, Transaction, TransferOperation, VoteOperation};

    #[test]
    fn transfer_operation_matches_dhive_vector() {
        let operation = Operation::Transfer(TransferOperation {
            from: "foo".to_string(),
            to: "bar".to_string(),
            amount: Asset::from_string("1.000 STEEM").expect("asset should parse"),
            memo: "wedding present".to_string(),
        });

        let mut buf = Vec::new();
        operation
            .hive_serialize(&mut buf)
            .expect("operation should serialize");
        assert_eq!(
            hex::encode(buf),
            "0203666f6f03626172e80300000000000003535445454d00000f77656464696e672070726573656e74"
        );
    }

    #[test]
    fn transaction_serialization_matches_dhive_vector() {
        let tx = Transaction {
            ref_block_num: 1234,
            ref_block_prefix: 1122334455,
            expiration: "2017-07-15T16:51:19".to_string(),
            operations: vec![Operation::Vote(VoteOperation {
                voter: "foo".to_string(),
                author: "bar".to_string(),
                permlink: "baz".to_string(),
                weight: 10000,
            })],
            extensions: vec!["long-pants".to_string()],
        };

        let bytes = serialize_transaction(&tx).expect("transaction should serialize");
        assert_eq!(
            hex::encode(bytes),
            "d204f776e54207486a59010003666f6f036261720362617a1027010a6c6f6e672d70616e7473"
        );
    }

    #[test]
    fn digest_and_id_match_dhive_vectors() {
        let tx = Transaction {
            ref_block_num: 1234,
            ref_block_prefix: 1122334455,
            expiration: "2017-07-15T16:51:19".to_string(),
            operations: vec![Operation::Vote(VoteOperation {
                voter: "foo".to_string(),
                author: "bar".to_string(),
                permlink: "baz".to_string(),
                weight: 10000,
            })],
            extensions: vec!["long-pants".to_string()],
        };

        let chain_id = ChainId { bytes: [0_u8; 32] };
        let digest = transaction_digest(&tx, &chain_id).expect("digest should compute");
        assert_eq!(
            hex::encode(digest),
            "77342bdde45a4901a0a65a98e0806a292ccfeb8b9b048d1ca93af69434c866de"
        );

        let trx_id = generate_trx_id(&tx).expect("trx id should compute");
        assert_eq!(trx_id, "70a8b9bd8e4a1413eb807f030fa8e81f9c7bb615");
    }
}
