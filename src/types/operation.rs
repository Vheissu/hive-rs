use std::collections::BTreeMap;

use serde::de::Error as _;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::types::{Asset, Authority, BeneficiaryRoute, ChainProperties, Price, SignedBlockHeader};

// Field declaration order in each operation struct is intentionally aligned with
// Hive's binary serializer order.
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Vote(VoteOperation),                                               // 0
    Comment(CommentOperation),                                         // 1
    Transfer(TransferOperation),                                       // 2
    TransferToVesting(TransferToVestingOperation),                     // 3
    WithdrawVesting(WithdrawVestingOperation),                         // 4
    LimitOrderCreate(LimitOrderCreateOperation),                       // 5
    LimitOrderCancel(LimitOrderCancelOperation),                       // 6
    FeedPublish(FeedPublishOperation),                                 // 7
    Convert(ConvertOperation),                                         // 8
    AccountCreate(AccountCreateOperation),                             // 9
    AccountUpdate(AccountUpdateOperation),                             // 10
    WitnessUpdate(WitnessUpdateOperation),                             // 11
    AccountWitnessVote(AccountWitnessVoteOperation),                   // 12
    AccountWitnessProxy(AccountWitnessProxyOperation),                 // 13
    Pow(PowOperation),                                                 // 14
    Custom(CustomOperation),                                           // 15
    ReportOverProduction(ReportOverProductionOperation),               // 16
    DeleteComment(DeleteCommentOperation),                             // 17
    CustomJson(CustomJsonOperation),                                   // 18
    CommentOptions(CommentOptionsOperation),                           // 19
    SetWithdrawVestingRoute(SetWithdrawVestingRouteOperation),         // 20
    LimitOrderCreate2(LimitOrderCreate2Operation),                     // 21
    ClaimAccount(ClaimAccountOperation),                               // 22
    CreateClaimedAccount(CreateClaimedAccountOperation),               // 23
    RequestAccountRecovery(RequestAccountRecoveryOperation),           // 24
    RecoverAccount(RecoverAccountOperation),                           // 25
    ChangeRecoveryAccount(ChangeRecoveryAccountOperation),             // 26
    EscrowTransfer(EscrowTransferOperation),                           // 27
    EscrowDispute(EscrowDisputeOperation),                             // 28
    EscrowRelease(EscrowReleaseOperation),                             // 29
    Pow2(Pow2Operation),                                               // 30
    EscrowApprove(EscrowApproveOperation),                             // 31
    TransferToSavings(TransferToSavingsOperation),                     // 32
    TransferFromSavings(TransferFromSavingsOperation),                 // 33
    CancelTransferFromSavings(CancelTransferFromSavingsOperation),     // 34
    CustomBinary(CustomBinaryOperation),                               // 35
    DeclineVotingRights(DeclineVotingRightsOperation),                 // 36
    ResetAccount(ResetAccountOperation),                               // 37
    SetResetAccount(SetResetAccountOperation),                         // 38
    ClaimRewardBalance(ClaimRewardBalanceOperation),                   // 39
    DelegateVestingShares(DelegateVestingSharesOperation),             // 40
    AccountCreateWithDelegation(AccountCreateWithDelegationOperation), // 41
    WitnessSetProperties(WitnessSetPropertiesOperation),               // 42
    AccountUpdate2(AccountUpdate2Operation),                           // 43
    CreateProposal(CreateProposalOperation),                           // 44
    UpdateProposalVotes(UpdateProposalVotesOperation),                 // 45
    RemoveProposal(RemoveProposalOperation),                           // 46
    UpdateProposal(UpdateProposalOperation),                           // 47
    CollateralizedConvert(CollateralizedConvertOperation),             // 48
    RecurrentTransfer(RecurrentTransferOperation),                     // 49
}

impl Operation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Vote(_) => "vote",
            Self::Comment(_) => "comment",
            Self::Transfer(_) => "transfer",
            Self::TransferToVesting(_) => "transfer_to_vesting",
            Self::WithdrawVesting(_) => "withdraw_vesting",
            Self::LimitOrderCreate(_) => "limit_order_create",
            Self::LimitOrderCancel(_) => "limit_order_cancel",
            Self::FeedPublish(_) => "feed_publish",
            Self::Convert(_) => "convert",
            Self::AccountCreate(_) => "account_create",
            Self::AccountUpdate(_) => "account_update",
            Self::WitnessUpdate(_) => "witness_update",
            Self::AccountWitnessVote(_) => "account_witness_vote",
            Self::AccountWitnessProxy(_) => "account_witness_proxy",
            Self::Pow(_) => "pow",
            Self::Custom(_) => "custom",
            Self::ReportOverProduction(_) => "report_over_production",
            Self::DeleteComment(_) => "delete_comment",
            Self::CustomJson(_) => "custom_json",
            Self::CommentOptions(_) => "comment_options",
            Self::SetWithdrawVestingRoute(_) => "set_withdraw_vesting_route",
            Self::LimitOrderCreate2(_) => "limit_order_create2",
            Self::ClaimAccount(_) => "claim_account",
            Self::CreateClaimedAccount(_) => "create_claimed_account",
            Self::RequestAccountRecovery(_) => "request_account_recovery",
            Self::RecoverAccount(_) => "recover_account",
            Self::ChangeRecoveryAccount(_) => "change_recovery_account",
            Self::EscrowTransfer(_) => "escrow_transfer",
            Self::EscrowDispute(_) => "escrow_dispute",
            Self::EscrowRelease(_) => "escrow_release",
            Self::Pow2(_) => "pow2",
            Self::EscrowApprove(_) => "escrow_approve",
            Self::TransferToSavings(_) => "transfer_to_savings",
            Self::TransferFromSavings(_) => "transfer_from_savings",
            Self::CancelTransferFromSavings(_) => "cancel_transfer_from_savings",
            Self::CustomBinary(_) => "custom_binary",
            Self::DeclineVotingRights(_) => "decline_voting_rights",
            Self::ResetAccount(_) => "reset_account",
            Self::SetResetAccount(_) => "set_reset_account",
            Self::ClaimRewardBalance(_) => "claim_reward_balance",
            Self::DelegateVestingShares(_) => "delegate_vesting_shares",
            Self::AccountCreateWithDelegation(_) => "account_create_with_delegation",
            Self::WitnessSetProperties(_) => "witness_set_properties",
            Self::AccountUpdate2(_) => "account_update2",
            Self::CreateProposal(_) => "create_proposal",
            Self::UpdateProposalVotes(_) => "update_proposal_votes",
            Self::RemoveProposal(_) => "remove_proposal",
            Self::UpdateProposal(_) => "update_proposal",
            Self::CollateralizedConvert(_) => "collateralized_convert",
            Self::RecurrentTransfer(_) => "recurrent_transfer",
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Self::Vote(_) => 0,
            Self::Comment(_) => 1,
            Self::Transfer(_) => 2,
            Self::TransferToVesting(_) => 3,
            Self::WithdrawVesting(_) => 4,
            Self::LimitOrderCreate(_) => 5,
            Self::LimitOrderCancel(_) => 6,
            Self::FeedPublish(_) => 7,
            Self::Convert(_) => 8,
            Self::AccountCreate(_) => 9,
            Self::AccountUpdate(_) => 10,
            Self::WitnessUpdate(_) => 11,
            Self::AccountWitnessVote(_) => 12,
            Self::AccountWitnessProxy(_) => 13,
            Self::Pow(_) => 14,
            Self::Custom(_) => 15,
            Self::ReportOverProduction(_) => 16,
            Self::DeleteComment(_) => 17,
            Self::CustomJson(_) => 18,
            Self::CommentOptions(_) => 19,
            Self::SetWithdrawVestingRoute(_) => 20,
            Self::LimitOrderCreate2(_) => 21,
            Self::ClaimAccount(_) => 22,
            Self::CreateClaimedAccount(_) => 23,
            Self::RequestAccountRecovery(_) => 24,
            Self::RecoverAccount(_) => 25,
            Self::ChangeRecoveryAccount(_) => 26,
            Self::EscrowTransfer(_) => 27,
            Self::EscrowDispute(_) => 28,
            Self::EscrowRelease(_) => 29,
            Self::Pow2(_) => 30,
            Self::EscrowApprove(_) => 31,
            Self::TransferToSavings(_) => 32,
            Self::TransferFromSavings(_) => 33,
            Self::CancelTransferFromSavings(_) => 34,
            Self::CustomBinary(_) => 35,
            Self::DeclineVotingRights(_) => 36,
            Self::ResetAccount(_) => 37,
            Self::SetResetAccount(_) => 38,
            Self::ClaimRewardBalance(_) => 39,
            Self::DelegateVestingShares(_) => 40,
            Self::AccountCreateWithDelegation(_) => 41,
            Self::WitnessSetProperties(_) => 42,
            Self::AccountUpdate2(_) => 43,
            Self::CreateProposal(_) => 44,
            Self::UpdateProposalVotes(_) => 45,
            Self::RemoveProposal(_) => 46,
            Self::UpdateProposal(_) => 47,
            Self::CollateralizedConvert(_) => 48,
            Self::RecurrentTransfer(_) => 49,
        }
    }
}

impl Serialize for Operation {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(self.name())?;
        match self {
            Self::Vote(op) => seq.serialize_element(op)?,
            Self::Comment(op) => seq.serialize_element(op)?,
            Self::Transfer(op) => seq.serialize_element(op)?,
            Self::TransferToVesting(op) => seq.serialize_element(op)?,
            Self::WithdrawVesting(op) => seq.serialize_element(op)?,
            Self::LimitOrderCreate(op) => seq.serialize_element(op)?,
            Self::LimitOrderCancel(op) => seq.serialize_element(op)?,
            Self::FeedPublish(op) => seq.serialize_element(op)?,
            Self::Convert(op) => seq.serialize_element(op)?,
            Self::AccountCreate(op) => seq.serialize_element(op)?,
            Self::AccountUpdate(op) => seq.serialize_element(op)?,
            Self::WitnessUpdate(op) => seq.serialize_element(op)?,
            Self::AccountWitnessVote(op) => seq.serialize_element(op)?,
            Self::AccountWitnessProxy(op) => seq.serialize_element(op)?,
            Self::Pow(op) => seq.serialize_element(op)?,
            Self::Custom(op) => seq.serialize_element(op)?,
            Self::ReportOverProduction(op) => seq.serialize_element(op)?,
            Self::DeleteComment(op) => seq.serialize_element(op)?,
            Self::CustomJson(op) => seq.serialize_element(op)?,
            Self::CommentOptions(op) => seq.serialize_element(op)?,
            Self::SetWithdrawVestingRoute(op) => seq.serialize_element(op)?,
            Self::LimitOrderCreate2(op) => seq.serialize_element(op)?,
            Self::ClaimAccount(op) => seq.serialize_element(op)?,
            Self::CreateClaimedAccount(op) => seq.serialize_element(op)?,
            Self::RequestAccountRecovery(op) => seq.serialize_element(op)?,
            Self::RecoverAccount(op) => seq.serialize_element(op)?,
            Self::ChangeRecoveryAccount(op) => seq.serialize_element(op)?,
            Self::EscrowTransfer(op) => seq.serialize_element(op)?,
            Self::EscrowDispute(op) => seq.serialize_element(op)?,
            Self::EscrowRelease(op) => seq.serialize_element(op)?,
            Self::Pow2(op) => seq.serialize_element(op)?,
            Self::EscrowApprove(op) => seq.serialize_element(op)?,
            Self::TransferToSavings(op) => seq.serialize_element(op)?,
            Self::TransferFromSavings(op) => seq.serialize_element(op)?,
            Self::CancelTransferFromSavings(op) => seq.serialize_element(op)?,
            Self::CustomBinary(op) => seq.serialize_element(op)?,
            Self::DeclineVotingRights(op) => seq.serialize_element(op)?,
            Self::ResetAccount(op) => seq.serialize_element(op)?,
            Self::SetResetAccount(op) => seq.serialize_element(op)?,
            Self::ClaimRewardBalance(op) => seq.serialize_element(op)?,
            Self::DelegateVestingShares(op) => seq.serialize_element(op)?,
            Self::AccountCreateWithDelegation(op) => seq.serialize_element(op)?,
            Self::WitnessSetProperties(op) => seq.serialize_element(op)?,
            Self::AccountUpdate2(op) => seq.serialize_element(op)?,
            Self::CreateProposal(op) => seq.serialize_element(op)?,
            Self::UpdateProposalVotes(op) => seq.serialize_element(op)?,
            Self::RemoveProposal(op) => seq.serialize_element(op)?,
            Self::UpdateProposal(op) => seq.serialize_element(op)?,
            Self::CollateralizedConvert(op) => seq.serialize_element(op)?,
            Self::RecurrentTransfer(op) => seq.serialize_element(op)?,
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Vec::<Value>::deserialize(deserializer)?;
        if value.len() != 2 {
            return Err(D::Error::custom("operation must be a 2-item array"));
        }

        let op_name = value[0]
            .as_str()
            .ok_or_else(|| D::Error::custom("operation name must be a string"))?;
        let op_value = value[1].clone();

        macro_rules! parse_variant {
            ($variant:ident, $ty:ty) => {
                serde_json::from_value::<$ty>(op_value.clone())
                    .map(Self::$variant)
                    .map_err(D::Error::custom)
            };
        }

        match op_name {
            "vote" => parse_variant!(Vote, VoteOperation),
            "comment" => parse_variant!(Comment, CommentOperation),
            "transfer" => parse_variant!(Transfer, TransferOperation),
            "transfer_to_vesting" => parse_variant!(TransferToVesting, TransferToVestingOperation),
            "withdraw_vesting" => parse_variant!(WithdrawVesting, WithdrawVestingOperation),
            "limit_order_create" => parse_variant!(LimitOrderCreate, LimitOrderCreateOperation),
            "limit_order_cancel" => parse_variant!(LimitOrderCancel, LimitOrderCancelOperation),
            "feed_publish" => parse_variant!(FeedPublish, FeedPublishOperation),
            "convert" => parse_variant!(Convert, ConvertOperation),
            "account_create" => parse_variant!(AccountCreate, AccountCreateOperation),
            "account_update" => parse_variant!(AccountUpdate, AccountUpdateOperation),
            "witness_update" => parse_variant!(WitnessUpdate, WitnessUpdateOperation),
            "account_witness_vote" => {
                parse_variant!(AccountWitnessVote, AccountWitnessVoteOperation)
            }
            "account_witness_proxy" => {
                parse_variant!(AccountWitnessProxy, AccountWitnessProxyOperation)
            }
            "pow" => parse_variant!(Pow, PowOperation),
            "custom" => parse_variant!(Custom, CustomOperation),
            "report_over_production" => {
                parse_variant!(ReportOverProduction, ReportOverProductionOperation)
            }
            "delete_comment" => parse_variant!(DeleteComment, DeleteCommentOperation),
            "custom_json" => parse_variant!(CustomJson, CustomJsonOperation),
            "comment_options" => parse_variant!(CommentOptions, CommentOptionsOperation),
            "set_withdraw_vesting_route" => {
                parse_variant!(SetWithdrawVestingRoute, SetWithdrawVestingRouteOperation)
            }
            "limit_order_create2" => parse_variant!(LimitOrderCreate2, LimitOrderCreate2Operation),
            "claim_account" => parse_variant!(ClaimAccount, ClaimAccountOperation),
            "create_claimed_account" => {
                parse_variant!(CreateClaimedAccount, CreateClaimedAccountOperation)
            }
            "request_account_recovery" => {
                parse_variant!(RequestAccountRecovery, RequestAccountRecoveryOperation)
            }
            "recover_account" => parse_variant!(RecoverAccount, RecoverAccountOperation),
            "change_recovery_account" => {
                parse_variant!(ChangeRecoveryAccount, ChangeRecoveryAccountOperation)
            }
            "escrow_transfer" => parse_variant!(EscrowTransfer, EscrowTransferOperation),
            "escrow_dispute" => parse_variant!(EscrowDispute, EscrowDisputeOperation),
            "escrow_release" => parse_variant!(EscrowRelease, EscrowReleaseOperation),
            "pow2" => parse_variant!(Pow2, Pow2Operation),
            "escrow_approve" => parse_variant!(EscrowApprove, EscrowApproveOperation),
            "transfer_to_savings" => parse_variant!(TransferToSavings, TransferToSavingsOperation),
            "transfer_from_savings" => {
                parse_variant!(TransferFromSavings, TransferFromSavingsOperation)
            }
            "cancel_transfer_from_savings" => {
                parse_variant!(
                    CancelTransferFromSavings,
                    CancelTransferFromSavingsOperation
                )
            }
            "custom_binary" => parse_variant!(CustomBinary, CustomBinaryOperation),
            "decline_voting_rights" => {
                parse_variant!(DeclineVotingRights, DeclineVotingRightsOperation)
            }
            "reset_account" => parse_variant!(ResetAccount, ResetAccountOperation),
            "set_reset_account" => parse_variant!(SetResetAccount, SetResetAccountOperation),
            "claim_reward_balance" => {
                parse_variant!(ClaimRewardBalance, ClaimRewardBalanceOperation)
            }
            "delegate_vesting_shares" => {
                parse_variant!(DelegateVestingShares, DelegateVestingSharesOperation)
            }
            "account_create_with_delegation" => {
                parse_variant!(
                    AccountCreateWithDelegation,
                    AccountCreateWithDelegationOperation
                )
            }
            "witness_set_properties" => {
                parse_variant!(WitnessSetProperties, WitnessSetPropertiesOperation)
            }
            "account_update2" => parse_variant!(AccountUpdate2, AccountUpdate2Operation),
            "create_proposal" => parse_variant!(CreateProposal, CreateProposalOperation),
            "update_proposal_votes" => {
                parse_variant!(UpdateProposalVotes, UpdateProposalVotesOperation)
            }
            "remove_proposal" => parse_variant!(RemoveProposal, RemoveProposalOperation),
            "update_proposal" => parse_variant!(UpdateProposal, UpdateProposalOperation),
            "collateralized_convert" => {
                parse_variant!(CollateralizedConvert, CollateralizedConvertOperation)
            }
            "recurrent_transfer" => parse_variant!(RecurrentTransfer, RecurrentTransferOperation),
            _ => Err(D::Error::custom(format!(
                "unsupported operation type '{op_name}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum OperationName {
    Vote = 0,
    Comment = 1,
    Transfer = 2,
    TransferToVesting = 3,
    WithdrawVesting = 4,
    LimitOrderCreate = 5,
    LimitOrderCancel = 6,
    FeedPublish = 7,
    Convert = 8,
    AccountCreate = 9,
    AccountUpdate = 10,
    WitnessUpdate = 11,
    AccountWitnessVote = 12,
    AccountWitnessProxy = 13,
    Pow = 14,
    Custom = 15,
    ReportOverProduction = 16,
    DeleteComment = 17,
    CustomJson = 18,
    CommentOptions = 19,
    SetWithdrawVestingRoute = 20,
    LimitOrderCreate2 = 21,
    ClaimAccount = 22,
    CreateClaimedAccount = 23,
    RequestAccountRecovery = 24,
    RecoverAccount = 25,
    ChangeRecoveryAccount = 26,
    EscrowTransfer = 27,
    EscrowDispute = 28,
    EscrowRelease = 29,
    Pow2 = 30,
    EscrowApprove = 31,
    TransferToSavings = 32,
    TransferFromSavings = 33,
    CancelTransferFromSavings = 34,
    CustomBinary = 35,
    DeclineVotingRights = 36,
    ResetAccount = 37,
    SetResetAccount = 38,
    ClaimRewardBalance = 39,
    DelegateVestingShares = 40,
    AccountCreateWithDelegation = 41,
    WitnessSetProperties = 42,
    AccountUpdate2 = 43,
    CreateProposal = 44,
    UpdateProposalVotes = 45,
    RemoveProposal = 46,
    UpdateProposal = 47,
    CollateralizedConvert = 48,
    RecurrentTransfer = 49,
}

impl OperationName {
    pub fn id(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoteOperation {
    pub voter: String,
    pub author: String,
    pub permlink: String,
    pub weight: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentOperation {
    pub parent_author: String,
    pub parent_permlink: String,
    pub author: String,
    pub permlink: String,
    pub title: String,
    pub body: String,
    pub json_metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferOperation {
    pub from: String,
    pub to: String,
    pub amount: Asset,
    pub memo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferToVestingOperation {
    pub from: String,
    pub to: String,
    pub amount: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WithdrawVestingOperation {
    pub account: String,
    pub vesting_shares: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LimitOrderCreateOperation {
    pub owner: String,
    pub orderid: u32,
    pub amount_to_sell: Asset,
    pub min_to_receive: Asset,
    pub fill_or_kill: bool,
    pub expiration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LimitOrderCancelOperation {
    pub owner: String,
    pub orderid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedPublishOperation {
    pub publisher: String,
    pub exchange_rate: Price,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConvertOperation {
    pub owner: String,
    pub requestid: u32,
    pub amount: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountCreateOperation {
    pub fee: Asset,
    pub creator: String,
    pub new_account_name: String,
    pub owner: Authority,
    pub active: Authority,
    pub posting: Authority,
    pub memo_key: String,
    pub json_metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountUpdateOperation {
    pub account: String,
    pub owner: Option<Authority>,
    pub active: Option<Authority>,
    pub posting: Option<Authority>,
    pub memo_key: String,
    pub json_metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessUpdateOperation {
    pub owner: String,
    pub url: String,
    pub block_signing_key: String,
    pub props: ChainProperties,
    pub fee: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountWitnessVoteOperation {
    pub account: String,
    pub witness: String,
    pub approve: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountWitnessProxyOperation {
    pub account: String,
    pub proxy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowOperation {
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomOperation {
    #[serde(default)]
    pub required_auths: Vec<String>,
    pub id: u16,
    #[serde(default)]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportOverProductionOperation {
    pub reporter: String,
    pub first_block: SignedBlockHeader,
    pub second_block: SignedBlockHeader,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteCommentOperation {
    pub author: String,
    pub permlink: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CustomJsonOperation {
    #[serde(default)]
    pub required_auths: Vec<String>,
    #[serde(default)]
    pub required_posting_auths: Vec<String>,
    pub id: String,
    pub json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentOptionsOperation {
    pub author: String,
    pub permlink: String,
    pub max_accepted_payout: Asset,
    pub percent_hbd: u16,
    pub allow_votes: bool,
    pub allow_curation_rewards: bool,
    #[serde(default)]
    pub extensions: Vec<CommentOptionsExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetWithdrawVestingRouteOperation {
    pub from_account: String,
    pub to_account: String,
    pub percent: u16,
    pub auto_vest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LimitOrderCreate2Operation {
    pub owner: String,
    pub orderid: u32,
    pub amount_to_sell: Asset,
    pub exchange_rate: Price,
    pub fill_or_kill: bool,
    pub expiration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimAccountOperation {
    pub creator: String,
    pub fee: Asset,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateClaimedAccountOperation {
    pub creator: String,
    pub new_account_name: String,
    pub owner: Authority,
    pub active: Authority,
    pub posting: Authority,
    pub memo_key: String,
    pub json_metadata: String,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestAccountRecoveryOperation {
    pub recovery_account: String,
    pub account_to_recover: String,
    pub new_owner_authority: Authority,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoverAccountOperation {
    pub account_to_recover: String,
    pub new_owner_authority: Authority,
    pub recent_owner_authority: Authority,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeRecoveryAccountOperation {
    pub account_to_recover: String,
    pub new_recovery_account: String,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EscrowTransferOperation {
    pub from: String,
    pub to: String,
    pub hbd_amount: Asset,
    pub hive_amount: Asset,
    pub escrow_id: u32,
    pub agent: String,
    pub fee: Asset,
    pub json_meta: String,
    pub ratification_deadline: String,
    pub escrow_expiration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EscrowDisputeOperation {
    pub from: String,
    pub to: String,
    pub agent: String,
    pub who: String,
    pub escrow_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EscrowReleaseOperation {
    pub from: String,
    pub to: String,
    pub agent: String,
    pub who: String,
    pub receiver: String,
    pub escrow_id: u32,
    pub hbd_amount: Asset,
    pub hive_amount: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pow2Operation {
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EscrowApproveOperation {
    pub from: String,
    pub to: String,
    pub agent: String,
    pub who: String,
    pub escrow_id: u32,
    pub approve: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferToSavingsOperation {
    pub from: String,
    pub to: String,
    pub amount: Asset,
    pub memo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferFromSavingsOperation {
    pub from: String,
    pub request_id: u32,
    pub to: String,
    pub amount: Asset,
    pub memo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CancelTransferFromSavingsOperation {
    pub from: String,
    pub request_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomBinaryOperation {
    #[serde(default)]
    pub required_owner_auths: Vec<String>,
    #[serde(default)]
    pub required_active_auths: Vec<String>,
    #[serde(default)]
    pub required_posting_auths: Vec<String>,
    #[serde(default)]
    pub required_auths: Vec<Authority>,
    pub id: String,
    #[serde(default)]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclineVotingRightsOperation {
    pub account: String,
    pub decline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResetAccountOperation {
    pub reset_account: String,
    pub account_to_reset: String,
    pub new_owner_authority: Authority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetResetAccountOperation {
    pub account: String,
    pub current_reset_account: String,
    pub reset_account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimRewardBalanceOperation {
    pub account: String,
    pub reward_hive: Asset,
    pub reward_hbd: Asset,
    pub reward_vests: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegateVestingSharesOperation {
    pub delegator: String,
    pub delegatee: String,
    pub vesting_shares: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountCreateWithDelegationOperation {
    pub fee: Asset,
    pub delegation: Asset,
    pub creator: String,
    pub new_account_name: String,
    pub owner: Authority,
    pub active: Authority,
    pub posting: Authority,
    pub memo_key: String,
    pub json_metadata: String,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessSetPropertiesOperation {
    pub owner: String,
    #[serde(default)]
    pub props: Vec<(String, Vec<u8>)>,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountUpdate2Operation {
    pub account: String,
    pub owner: Option<Authority>,
    pub active: Option<Authority>,
    pub posting: Option<Authority>,
    pub memo_key: Option<String>,
    pub json_metadata: String,
    pub posting_json_metadata: String,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateProposalOperation {
    pub creator: String,
    pub receiver: String,
    pub start_date: String,
    pub end_date: String,
    pub daily_pay: Asset,
    pub subject: String,
    pub permlink: String,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateProposalVotesOperation {
    pub voter: String,
    #[serde(default)]
    pub proposal_ids: Vec<i64>,
    pub approve: bool,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoveProposalOperation {
    pub proposal_owner: String,
    #[serde(default)]
    pub proposal_ids: Vec<i64>,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateProposalOperation {
    pub proposal_id: u64,
    pub creator: String,
    pub daily_pay: Asset,
    pub subject: String,
    pub permlink: String,
    #[serde(default)]
    pub extensions: Vec<UpdateProposalExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollateralizedConvertOperation {
    pub owner: String,
    pub requestid: u32,
    pub amount: Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecurrentTransferOperation {
    pub from: String,
    pub to: String,
    pub amount: Asset,
    pub memo: String,
    pub recurrence: u16,
    pub executions: u16,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum CommentOptionsExtension {
    Beneficiaries {
        beneficiaries: Vec<BeneficiaryRoute>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum UpdateProposalExtension {
    Void,
    EndDate { end_date: String },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Operation, OperationName, TransferOperation};
    use crate::types::Asset;

    #[test]
    fn operation_tuple_format_round_trip() {
        let op = Operation::Transfer(TransferOperation {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: Asset::from_string("1.000 HIVE").expect("asset should parse"),
            memo: "hello".to_string(),
        });

        let serialized = serde_json::to_value(&op).expect("operation should serialize");
        assert_eq!(
            serialized,
            json!([
                "transfer",
                {
                    "from": "alice",
                    "to": "bob",
                    "amount": "1.000 HIVE",
                    "memo": "hello"
                }
            ])
        );

        let parsed: Operation = serde_json::from_value(serialized).expect("operation should parse");
        match parsed {
            Operation::Transfer(value) => {
                assert_eq!(value.from, "alice");
                assert_eq!(value.to, "bob");
            }
            _ => panic!("expected transfer operation"),
        }
    }

    #[test]
    fn operation_name_ids_match_expected_values() {
        let ids = [
            OperationName::Vote.id(),
            OperationName::Transfer.id(),
            OperationName::CustomJson.id(),
            OperationName::WitnessSetProperties.id(),
            OperationName::RecurrentTransfer.id(),
        ];
        assert_eq!(ids, [0, 2, 18, 42, 49]);
    }
}
