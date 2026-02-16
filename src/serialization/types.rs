use chrono::{DateTime, Utc};

use crate::crypto::keys::PublicKey;
use crate::error::{HiveError, Result};
use crate::types::{Asset, Authority, ChainProperties, Price};

pub fn write_u8(buf: &mut Vec<u8>, val: u8) {
    buf.push(val);
}

pub fn write_u16(buf: &mut Vec<u8>, val: u16) {
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn write_u32(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn write_u64(buf: &mut Vec<u8>, val: u64) {
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn write_i8(buf: &mut Vec<u8>, val: i8) {
    buf.push(val as u8);
}

pub fn write_i16(buf: &mut Vec<u8>, val: i16) {
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn write_i32(buf: &mut Vec<u8>, val: i32) {
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn write_i64(buf: &mut Vec<u8>, val: i64) {
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn write_varint32(buf: &mut Vec<u8>, mut val: u32) {
    while val >= 0x80 {
        buf.push(((val & 0x7F) as u8) | 0x80);
        val >>= 7;
    }
    buf.push(val as u8);
}

pub fn write_bool(buf: &mut Vec<u8>, val: bool) {
    buf.push(u8::from(val));
}

pub fn write_string(buf: &mut Vec<u8>, val: &str) {
    let bytes = val.as_bytes();
    write_varint32(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}

pub fn write_date(buf: &mut Vec<u8>, date: &str) -> Result<()> {
    let date_with_z = if date.ends_with('Z') {
        date.to_string()
    } else {
        format!("{date}Z")
    };

    let parsed = DateTime::parse_from_rfc3339(&date_with_z)
        .map_err(|err| HiveError::Serialization(format!("invalid date '{date}': {err}")))?;
    let timestamp = parsed.timestamp();
    if !(0..=u32::MAX as i64).contains(&timestamp) {
        return Err(HiveError::Serialization(format!(
            "date '{date}' is out of u32 timestamp range"
        )));
    }
    write_u32(buf, timestamp as u32);
    Ok(())
}

pub fn write_public_key(buf: &mut Vec<u8>, key: &str) -> Result<()> {
    let public = PublicKey::from_string(key)?;
    buf.extend_from_slice(&public.compressed_bytes());
    Ok(())
}

pub fn write_asset(buf: &mut Vec<u8>, asset: &Asset) -> Result<()> {
    let (amount, precision, symbol) = asset.steem_symbols();
    write_i64(buf, amount);
    write_u8(buf, precision);

    if symbol.len() > 7 {
        return Err(HiveError::Serialization(format!(
            "asset symbol '{symbol}' exceeds 7 bytes"
        )));
    }

    let mut symbol_bytes = [0_u8; 7];
    for (idx, byte) in symbol.as_bytes().iter().enumerate() {
        symbol_bytes[idx] = *byte;
    }
    buf.extend_from_slice(&symbol_bytes);
    Ok(())
}

pub fn write_optional<T, F>(buf: &mut Vec<u8>, opt: Option<&T>, mut serialize: F) -> Result<()>
where
    F: FnMut(&mut Vec<u8>, &T) -> Result<()>,
{
    match opt {
        Some(value) => {
            write_u8(buf, 1);
            serialize(buf, value)?;
        }
        None => write_u8(buf, 0),
    }
    Ok(())
}

pub fn write_array<T, F>(buf: &mut Vec<u8>, items: &[T], mut serialize: F) -> Result<()>
where
    F: FnMut(&mut Vec<u8>, &T) -> Result<()>,
{
    write_varint32(buf, items.len() as u32);
    for item in items {
        serialize(buf, item)?;
    }
    Ok(())
}

pub fn write_flat_map<K, V, FK, FV>(
    buf: &mut Vec<u8>,
    pairs: &[(K, V)],
    mut serialize_key: FK,
    mut serialize_val: FV,
) -> Result<()>
where
    FK: FnMut(&mut Vec<u8>, &K) -> Result<()>,
    FV: FnMut(&mut Vec<u8>, &V) -> Result<()>,
{
    write_varint32(buf, pairs.len() as u32);
    for (key, value) in pairs {
        serialize_key(buf, key)?;
        serialize_val(buf, value)?;
    }
    Ok(())
}

pub fn write_authority(buf: &mut Vec<u8>, authority: &Authority) -> Result<()> {
    write_u32(buf, authority.weight_threshold);
    write_flat_map(
        buf,
        &authority.account_auths,
        |b, account| {
            write_string(b, account);
            Ok(())
        },
        |b, weight| {
            write_u16(b, *weight);
            Ok(())
        },
    )?;
    write_flat_map(
        buf,
        &authority.key_auths,
        |b, key| write_public_key(b, key),
        |b, weight| {
            write_u16(b, *weight);
            Ok(())
        },
    )
}

pub fn write_price(buf: &mut Vec<u8>, price: &Price) -> Result<()> {
    write_asset(buf, &price.base)?;
    write_asset(buf, &price.quote)
}

pub fn write_chain_properties(buf: &mut Vec<u8>, props: &ChainProperties) -> Result<()> {
    write_asset(buf, &props.account_creation_fee)?;
    write_u32(buf, props.maximum_block_size);
    write_u16(buf, props.hbd_interest_rate);
    Ok(())
}

pub fn write_void_array(buf: &mut Vec<u8>) {
    write_varint32(buf, 0);
}

pub fn write_variable_binary(buf: &mut Vec<u8>, data: &[u8]) {
    write_varint32(buf, data.len() as u32);
    buf.extend_from_slice(data);
}

pub fn read_string(cursor: &mut &[u8]) -> Result<String> {
    let len = read_varint32(cursor)? as usize;
    if cursor.len() < len {
        return Err(HiveError::Serialization(
            "buffer shorter than encoded string length".to_string(),
        ));
    }
    let value = String::from_utf8(cursor[..len].to_vec())
        .map_err(|err| HiveError::Serialization(format!("invalid UTF-8 string: {err}")))?;
    *cursor = &cursor[len..];
    Ok(value)
}

pub fn read_varint32(cursor: &mut &[u8]) -> Result<u32> {
    let mut value = 0_u32;
    let mut shift = 0_u32;
    let mut index = 0_usize;

    while index < cursor.len() {
        let byte = cursor[index];
        value |= ((byte & 0x7F) as u32) << shift;
        index += 1;
        if byte & 0x80 == 0 {
            *cursor = &cursor[index..];
            return Ok(value);
        }
        shift += 7;
        if shift > 28 {
            return Err(HiveError::Serialization(
                "varint32 value is too large".to_string(),
            ));
        }
    }

    Err(HiveError::Serialization(
        "unexpected EOF while parsing varint32".to_string(),
    ))
}

pub fn parse_hive_time(value: &str) -> Result<DateTime<Utc>> {
    let date_with_z = if value.ends_with('Z') {
        value.to_string()
    } else {
        format!("{value}Z")
    };
    let parsed = DateTime::parse_from_rfc3339(&date_with_z)
        .map_err(|err| HiveError::Serialization(format!("invalid hive time '{value}': {err}")))?;
    Ok(parsed.with_timezone(&Utc))
}

pub fn format_hive_time(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use crate::serialization::types::{
        read_string, read_varint32, write_date, write_string, write_varint32,
    };

    #[test]
    fn varint_round_trip() {
        let values = [0_u32, 1, 127, 128, 255, 300, u16::MAX as u32, 1_000_000];
        for value in values {
            let mut buf = Vec::new();
            write_varint32(&mut buf, value);
            let mut slice = buf.as_slice();
            let decoded = read_varint32(&mut slice).expect("varint should decode");
            assert_eq!(decoded, value);
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn date_matches_known_vectors() {
        let mut buf = Vec::new();
        write_date(&mut buf, "2017-07-15T16:51:19").expect("date should serialize");
        assert_eq!(hex::encode(buf), "07486a59");

        let mut buf2 = Vec::new();
        write_date(&mut buf2, "2000-01-01T00:00:00").expect("date should serialize");
        assert_eq!(hex::encode(buf2), "80436d38");
    }

    #[test]
    fn string_round_trip() {
        let mut buf = Vec::new();
        write_string(&mut buf, "Hellooo fröm Swäden!");
        assert_eq!(
            hex::encode(&buf),
            "1648656c6c6f6f6f206672c3b66d205377c3a464656e21"
        );

        let mut slice = buf.as_slice();
        let decoded = read_string(&mut slice).expect("string should deserialize");
        assert_eq!(decoded, "Hellooo fröm Swäden!");
        assert!(slice.is_empty());
    }
}
