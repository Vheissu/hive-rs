use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{HiveError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetSymbol {
    Hive,
    Hbd,
    Vests,
    Custom(String),
}

impl AssetSymbol {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Hive => "HIVE",
            Self::Hbd => "HBD",
            Self::Vests => "VESTS",
            Self::Custom(symbol) => symbol.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Asset {
    pub amount: i64,
    pub precision: u8,
    pub symbol: AssetSymbol,
}

impl Asset {
    pub fn hive(amount: f64) -> Self {
        Self::from_float(amount, 3, AssetSymbol::Hive)
    }

    pub fn hbd(amount: f64) -> Self {
        Self::from_float(amount, 3, AssetSymbol::Hbd)
    }

    pub fn vests(amount: f64) -> Self {
        Self::from_float(amount, 6, AssetSymbol::Vests)
    }

    pub fn from_string(value: &str) -> Result<Self> {
        let mut parts = value.split_whitespace();
        let amount_raw = parts
            .next()
            .ok_or_else(|| HiveError::InvalidAsset("missing amount in asset string".to_string()))?;
        let symbol_raw = parts
            .next()
            .ok_or_else(|| HiveError::InvalidAsset("missing symbol in asset string".to_string()))?;

        if parts.next().is_some() {
            return Err(HiveError::InvalidAsset(
                "asset string must be '<amount> <symbol>'".to_string(),
            ));
        }

        let symbol_upper = symbol_raw.to_ascii_uppercase();
        let expected_precision = known_symbol_precision(&symbol_upper);
        let precision = parse_precision(amount_raw)?;

        if let Some(expected) = expected_precision {
            if precision != expected {
                return Err(HiveError::InvalidAsset(format!(
                    "symbol {symbol_upper} expects precision {expected}, got {precision}"
                )));
            }
        }

        let amount = parse_amount(amount_raw, precision)?;
        let symbol = match symbol_upper.as_str() {
            "HIVE" | "STEEM" | "TESTS" => AssetSymbol::Hive,
            "HBD" | "SBD" | "TBD" => AssetSymbol::Hbd,
            "VESTS" => AssetSymbol::Vests,
            _ => AssetSymbol::Custom(symbol_upper),
        };

        Ok(Self {
            amount,
            precision,
            symbol,
        })
    }

    pub fn steem_symbols(&self) -> (i64, u8, &str) {
        let symbol = match &self.symbol {
            AssetSymbol::Hive => "STEEM",
            AssetSymbol::Hbd => "SBD",
            AssetSymbol::Vests => "VESTS",
            AssetSymbol::Custom(symbol) => symbol.as_str(),
        };

        (self.amount, self.precision, symbol)
    }

    fn from_float(amount: f64, precision: u8, symbol: AssetSymbol) -> Self {
        let scale = 10_i64.pow(precision as u32);
        let amount = (amount * scale as f64).round() as i64;
        Self {
            amount,
            precision,
            symbol,
        }
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let scale = 10_i64.pow(self.precision as u32);
        let sign = if self.amount < 0 { "-" } else { "" };
        let abs = self.amount.unsigned_abs();
        let whole = abs / scale as u64;
        let fraction = abs % scale as u64;

        if self.precision == 0 {
            write!(f, "{sign}{whole} {}", self.symbol.as_str())
        } else {
            write!(
                f,
                "{sign}{whole}.{fraction:0width$} {}",
                self.symbol.as_str(),
                width = self.precision as usize
            )
        }
    }
}

impl FromStr for Asset {
    type Err = HiveError;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s)
    }
}

impl Serialize for Asset {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Asset {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_string(&value).map_err(D::Error::custom)
    }
}

fn known_symbol_precision(symbol: &str) -> Option<u8> {
    match symbol {
        "HIVE" | "HBD" | "STEEM" | "SBD" | "TESTS" | "TBD" => Some(3),
        "VESTS" => Some(6),
        _ => None,
    }
}

fn parse_precision(amount: &str) -> Result<u8> {
    let mut value = amount.trim();
    if let Some(stripped) = value.strip_prefix('+') {
        value = stripped;
    }
    if let Some(stripped) = value.strip_prefix('-') {
        value = stripped;
    }
    let precision = match value.split_once('.') {
        Some((_, fractional)) => fractional.len(),
        None => 0,
    };
    u8::try_from(precision).map_err(|_| HiveError::InvalidAsset("invalid precision".to_string()))
}

fn parse_amount(raw: &str, precision: u8) -> Result<i64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(HiveError::InvalidAsset(
            "amount cannot be empty".to_string(),
        ));
    }

    let (negative, body) = if let Some(value) = raw.strip_prefix('-') {
        (true, value)
    } else if let Some(value) = raw.strip_prefix('+') {
        (false, value)
    } else {
        (false, raw)
    };

    if body.is_empty() {
        return Err(HiveError::InvalidAsset("invalid amount".to_string()));
    }

    let (whole_raw, fractional_raw) = match body.split_once('.') {
        Some(parts) => {
            if parts.0.contains('.') || parts.1.contains('.') {
                return Err(HiveError::InvalidAsset("invalid amount format".to_string()));
            }
            (parts.0, parts.1)
        }
        None => (body, ""),
    };

    if !whole_raw.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(HiveError::InvalidAsset(
            "whole amount contains non-digit characters".to_string(),
        ));
    }

    if !fractional_raw.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(HiveError::InvalidAsset(
            "fractional amount contains non-digit characters".to_string(),
        ));
    }

    let expected_fraction_len = precision as usize;
    if fractional_raw.len() != expected_fraction_len {
        return Err(HiveError::InvalidAsset(format!(
            "expected {expected_fraction_len} decimal places, got {}",
            fractional_raw.len()
        )));
    }

    let scale = 10_i128
        .checked_pow(precision as u32)
        .ok_or_else(|| HiveError::InvalidAsset("precision out of range".to_string()))?;
    let whole = whole_raw
        .parse::<i128>()
        .map_err(|_| HiveError::InvalidAsset("invalid whole amount".to_string()))?;
    let fractional = if fractional_raw.is_empty() {
        0_i128
    } else {
        fractional_raw
            .parse::<i128>()
            .map_err(|_| HiveError::InvalidAsset("invalid fractional amount".to_string()))?
    };

    let mut amount = whole
        .checked_mul(scale)
        .and_then(|base| base.checked_add(fractional))
        .ok_or_else(|| HiveError::InvalidAsset("asset amount overflow".to_string()))?;

    if negative {
        amount = amount
            .checked_neg()
            .ok_or_else(|| HiveError::InvalidAsset("asset amount overflow".to_string()))?;
    }

    i64::try_from(amount).map_err(|_| HiveError::InvalidAsset("asset amount overflow".to_string()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Asset, AssetSymbol};

    #[test]
    fn parse_and_round_trip_canonical_assets() {
        let hive = Asset::from_string("1.000 HIVE").expect("hive asset should parse");
        let hbd = Asset::from_string("0.001 HBD").expect("hbd asset should parse");
        let vests = Asset::from_string("123456.789000 VESTS").expect("vests asset should parse");

        assert_eq!(hive.to_string(), "1.000 HIVE");
        assert_eq!(hbd.to_string(), "0.001 HBD");
        assert_eq!(vests.to_string(), "123456.789000 VESTS");
    }

    #[test]
    fn parses_negative_legacy_sbd_symbol() {
        let asset = Asset::from_string("-100.333 SBD").expect("negative sbd should parse");
        assert_eq!(asset.amount, -100_333);
        assert_eq!(asset.precision, 3);
        assert_eq!(asset.symbol, AssetSymbol::Hbd);
        assert_eq!(asset.to_string(), "-100.333 HBD");
    }

    #[test]
    fn serde_json_round_trip() {
        let input = Asset::from_string("42.123 HIVE").expect("asset should parse");
        let serialized = serde_json::to_value(&input).expect("asset should serialize");
        assert_eq!(serialized, json!("42.123 HIVE"));

        let deserialized: Asset =
            serde_json::from_value(serialized).expect("asset should deserialize");
        assert_eq!(deserialized, input);
    }

    #[test]
    fn steem_symbol_mapping() {
        let hive = Asset::from_string("1.000 HIVE").expect("asset should parse");
        let hbd = Asset::from_string("2.000 HBD").expect("asset should parse");
        let vests = Asset::from_string("3.000000 VESTS").expect("asset should parse");

        assert_eq!(hive.steem_symbols(), (1_000, 3, "STEEM"));
        assert_eq!(hbd.steem_symbols(), (2_000, 3, "SBD"));
        assert_eq!(vests.steem_symbols(), (3_000_000, 6, "VESTS"));
    }
}
