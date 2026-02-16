use crate::types::{Asset, AssetSymbol, DynamicGlobalProperties, Price};

pub fn get_vesting_share_price(props: &DynamicGlobalProperties) -> Price {
    let base = props
        .total_vesting_fund_hive
        .clone()
        .unwrap_or_else(|| Asset::hive(0.0));
    let quote = props
        .total_vesting_shares
        .clone()
        .unwrap_or_else(|| Asset::vests(0.0));

    Price { base, quote }
}

pub fn get_vests(props: &DynamicGlobalProperties, hive_power: &Asset) -> Asset {
    let fund = match props.total_vesting_fund_hive.as_ref() {
        Some(value) if value.amount != 0 => value,
        _ => return Asset::vests(0.0),
    };

    let shares = match props.total_vesting_shares.as_ref() {
        Some(value) => value,
        None => return Asset::vests(0.0),
    };

    let amount =
        ((hive_power.amount as f64) * (shares.amount as f64) / (fund.amount as f64)).round() as i64;
    Asset {
        amount,
        precision: 6,
        symbol: AssetSymbol::Vests,
    }
}
