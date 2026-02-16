mod asset_helpers;
mod nonce;

use serde_json::Value;

use crate::error::{HiveError, Result};
use crate::serialization::types::{
    write_asset, write_price, write_public_key, write_string, write_u16, write_u32,
};
use crate::types::OperationName;
use crate::types::{Asset, Price, WitnessProps, WitnessSetPropertiesOperation};

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

pub fn build_witness_update_op(
    owner: &str,
    props: WitnessProps,
) -> Result<WitnessSetPropertiesOperation> {
    let mut serialized_props = Vec::new();

    for (key, value) in props.extra {
        let mut buf = Vec::new();
        match key.as_str() {
            "key" | "new_signing_key" => {
                let key_str = value
                    .as_str()
                    .ok_or_else(|| HiveError::Serialization(format!("{key} must be a string")))?;
                write_public_key(&mut buf, key_str)?;
            }
            "account_subsidy_budget" | "account_subsidy_decay" | "maximum_block_size" => {
                let number = parse_u32(&value, &key)?;
                write_u32(&mut buf, number);
            }
            "hbd_interest_rate" => {
                let number = parse_u16(&value, &key)?;
                write_u16(&mut buf, number);
            }
            "url" => {
                let url = value
                    .as_str()
                    .ok_or_else(|| HiveError::Serialization("url must be a string".to_string()))?;
                write_string(&mut buf, url);
            }
            "hbd_exchange_rate" => {
                let price: Price = serde_json::from_value(value).map_err(HiveError::from)?;
                write_price(&mut buf, &price)?;
            }
            "account_creation_fee" => {
                let fee: Asset = serde_json::from_value(value).map_err(HiveError::from)?;
                write_asset(&mut buf, &fee)?;
            }
            _ => {
                return Err(HiveError::Serialization(format!(
                    "unknown witness prop: {key}"
                )));
            }
        }

        serialized_props.push((key, buf));
    }

    serialized_props.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(WitnessSetPropertiesOperation {
        owner: owner.to_string(),
        props: serialized_props,
        extensions: vec![],
    })
}

fn parse_u32(value: &Value, field: &str) -> Result<u32> {
    let Some(number) = value.as_u64() else {
        return Err(HiveError::Serialization(format!(
            "{field} must be an unsigned integer"
        )));
    };
    u32::try_from(number)
        .map_err(|_| HiveError::Serialization(format!("{field} is out of u32 range")))
}

fn parse_u16(value: &Value, field: &str) -> Result<u16> {
    let Some(number) = value.as_u64() else {
        return Err(HiveError::Serialization(format!(
            "{field} must be an unsigned integer"
        )));
    };
    u16::try_from(number)
        .map_err(|_| HiveError::Serialization(format!("{field} is out of u16 range")))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::types::{OperationName, WitnessProps};
    use crate::utils::{build_witness_update_op, make_bit_mask_filter};

    #[test]
    fn make_bitmask_filter_sets_expected_bits() {
        let (low, high) = make_bit_mask_filter(&[
            OperationName::Vote,
            OperationName::CustomJson,
            OperationName::RecurrentTransfer,
        ]);
        assert_eq!(low & (1 << 0), 1 << 0);
        assert_eq!(low & (1 << 18), 1 << 18);
        assert_eq!(low & (1 << 49), 1 << 49);
        assert_eq!(high, 0);
    }

    #[test]
    fn build_witness_update_op_serializes_and_sorts_props() {
        let mut props = WitnessProps::default();
        props
            .extra
            .insert("url".to_string(), json!("https://example.com"));
        props
            .extra
            .insert("hbd_interest_rate".to_string(), json!(1000));

        let operation = build_witness_update_op("alice", props).expect("op should build");
        assert_eq!(operation.owner, "alice");
        assert_eq!(operation.props.len(), 2);
        assert_eq!(operation.props[0].0, "hbd_interest_rate");
        assert_eq!(operation.props[1].0, "url");
    }
}
