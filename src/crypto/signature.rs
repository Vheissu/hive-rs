use crate::error::{HiveError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature {
    pub data: [u8; 65],
}

impl Signature {
    pub fn from_bytes(data: [u8; 65]) -> Self {
        Self { data }
    }

    pub fn from_hex(value: &str) -> Result<Self> {
        let bytes = hex::decode(value)
            .map_err(|err| HiveError::Signing(format!("invalid signature hex: {err}")))?;
        let data: [u8; 65] = bytes
            .try_into()
            .map_err(|_| HiveError::Signing("signature must be 65 bytes".to_string()))?;
        Ok(Self { data })
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.data)
    }

    #[allow(clippy::nonminimal_bool)]
    pub fn is_canonical(&self) -> bool {
        let sig = &self.data[1..65];
        !(sig[0] & 0x80 != 0)
            && !(sig[0] == 0 && sig[1] & 0x80 == 0)
            && !(sig[32] & 0x80 != 0)
            && !(sig[32] == 0 && sig[33] & 0x80 == 0)
    }
}
