use crate::error::{HiveError, Result};
use crate::serialization::types::read_varint32;

pub trait HiveDeserialize: Sized {
    fn hive_deserialize(cursor: &mut &[u8]) -> Result<Self>;
}

pub fn read_u8(cursor: &mut &[u8]) -> Result<u8> {
    if cursor.is_empty() {
        return Err(HiveError::Serialization(
            "buffer underflow for u8".to_string(),
        ));
    }
    let value = cursor[0];
    *cursor = &cursor[1..];
    Ok(value)
}

pub fn read_u16(cursor: &mut &[u8]) -> Result<u16> {
    if cursor.len() < 2 {
        return Err(HiveError::Serialization(
            "buffer underflow for u16".to_string(),
        ));
    }
    let value = u16::from_le_bytes(
        cursor[..2]
            .try_into()
            .map_err(|_| HiveError::Serialization("invalid u16 bytes".to_string()))?,
    );
    *cursor = &cursor[2..];
    Ok(value)
}

pub fn read_u32(cursor: &mut &[u8]) -> Result<u32> {
    if cursor.len() < 4 {
        return Err(HiveError::Serialization(
            "buffer underflow for u32".to_string(),
        ));
    }
    let value = u32::from_le_bytes(
        cursor[..4]
            .try_into()
            .map_err(|_| HiveError::Serialization("invalid u32 bytes".to_string()))?,
    );
    *cursor = &cursor[4..];
    Ok(value)
}

pub fn read_u64(cursor: &mut &[u8]) -> Result<u64> {
    if cursor.len() < 8 {
        return Err(HiveError::Serialization(
            "buffer underflow for u64".to_string(),
        ));
    }
    let value = u64::from_le_bytes(
        cursor[..8]
            .try_into()
            .map_err(|_| HiveError::Serialization("invalid u64 bytes".to_string()))?,
    );
    *cursor = &cursor[8..];
    Ok(value)
}

pub fn read_variable_binary(cursor: &mut &[u8]) -> Result<Vec<u8>> {
    let len = read_varint32(cursor)? as usize;
    if cursor.len() < len {
        return Err(HiveError::Serialization(
            "buffer underflow for variable binary".to_string(),
        ));
    }
    let value = cursor[..len].to_vec();
    *cursor = &cursor[len..];
    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::serialization::deserializer::{
        read_u16, read_u32, read_u64, read_u8, read_variable_binary,
    };
    use crate::serialization::types::write_variable_binary;

    #[test]
    fn reads_little_endian_primitives() {
        let mut bytes = [0x11_u8, 0x22, 0x33, 0x44, 0x55].as_slice();
        assert_eq!(read_u8(&mut bytes).expect("read u8"), 0x11);
        assert_eq!(read_u16(&mut bytes).expect("read u16"), 0x3322);
        assert_eq!(read_u16(&mut bytes).expect("read u16"), 0x5544);
    }

    #[test]
    fn reads_u32_and_u64() {
        let mut bytes = [
            0x78_u8, 0x56, 0x34, 0x12, // u32
            0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE, // u64
        ]
        .as_slice();
        assert_eq!(read_u32(&mut bytes).expect("read u32"), 0x12345678);
        assert_eq!(read_u64(&mut bytes).expect("read u64"), 0xFEDCBA9876543210);
    }

    #[test]
    fn reads_variable_binary() {
        let mut encoded = Vec::new();
        write_variable_binary(&mut encoded, b"hello");
        let mut cursor = encoded.as_slice();
        let value = read_variable_binary(&mut cursor).expect("read variable binary");
        assert_eq!(value, b"hello");
        assert!(cursor.is_empty());
    }
}
