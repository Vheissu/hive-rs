mod asset_helpers;
mod nonce;

use crate::types::OperationName;

pub use asset_helpers::{get_vesting_share_price, get_vests};
pub use nonce::unique_nonce;

pub fn make_bit_mask_filter(operations: &[OperationName]) -> (u64, u64) {
    let mut lower = 0_u64;
    let mut upper = 0_u64;

    for operation in operations {
        let id = operation.id();
        if id < 64 {
            lower |= 1_u64 << id;
        } else {
            upper |= 1_u64 << (id - 64);
        }
    }

    (lower, upper)
}
