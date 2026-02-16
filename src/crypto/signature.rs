use secp256k1::{Message, Secp256k1};

use crate::crypto::keys::{recoverable_from_signature, PublicKey};
use crate::error::{HiveError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature {
    pub data: [u8; 65],
}

impl Signature {
    pub fn from_bytes(data: [u8; 65]) -> Self {
        Self { data }
    }

    pub fn from_compact(compact: [u8; 64], recovery_id: u8) -> Result<Self> {
        if recovery_id > 3 {
            return Err(HiveError::Signing(format!(
                "invalid recovery id {recovery_id}"
            )));
        }
        let mut data = [0_u8; 65];
        data[0] = recovery_id + 31;
        data[1..].copy_from_slice(&compact);
        Ok(Self { data })
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

    pub fn compact_bytes(&self) -> [u8; 64] {
        self.data[1..65]
            .try_into()
            .expect("slice length is guaranteed")
    }

    pub fn recovery_id(&self) -> u8 {
        self.data[0].saturating_sub(31)
    }

    #[allow(clippy::nonminimal_bool)]
    pub fn is_canonical(&self) -> bool {
        Self::is_canonical_compact(&self.compact_bytes())
    }

    #[allow(clippy::nonminimal_bool)]
    pub fn is_canonical_compact(signature: &[u8; 64]) -> bool {
        !(signature[0] & 0x80 != 0)
            && !(signature[0] == 0 && signature[1] & 0x80 == 0)
            && !(signature[32] & 0x80 != 0)
            && !(signature[32] == 0 && signature[33] & 0x80 == 0)
    }

    pub fn recover(&self, digest: &[u8; 32]) -> Result<PublicKey> {
        let recoverable = recoverable_from_signature(self)?;
        let message = Message::from_digest_slice(digest)
            .map_err(|err| HiveError::Signing(format!("invalid digest: {err}")))?;
        let secp = Secp256k1::verification_only();
        let key = secp
            .recover_ecdsa(&message, &recoverable)
            .map_err(|err| HiveError::Signing(format!("recover failed: {err}")))?;
        Ok(PublicKey::from_secp256k1(key, "STM"))
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::keys::{KeyRole, PrivateKey};
    use crate::crypto::signature::Signature;

    #[test]
    fn sign_and_recover_matches_known_vector() {
        let key = PrivateKey::from_login("foo", "barman", KeyRole::Active).expect("valid key");
        let digest =
            hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
                .expect("hex should decode");
        let digest: [u8; 32] = digest.try_into().expect("digest length must be 32");

        let signature = key.sign(&digest).expect("signing should succeed");
        assert_eq!(
            signature.to_hex(),
            "20173e52773241c69a8870c796634a537cb543e088c8aa13b89d46e33c0227c62e4afda5266272bd53c4e3e7f417af4d811b3fae5bd069c94447f1fdc48a525b8d"
        );
        assert!(signature.is_canonical());

        let recovered = signature.recover(&digest).expect("recovery should succeed");
        assert_eq!(
            recovered.to_string(),
            "STM87F7tN56tAUL2C6J9Gzi9HzgNpZdi6M2cLQo7TjDU5v178QsYA"
        );
        assert!(recovered.verify(&digest, &signature));
    }

    #[test]
    fn signature_hex_round_trip() {
        let hex = "20173e52773241c69a8870c796634a537cb543e088c8aa13b89d46e33c0227c62e4afda5266272bd53c4e3e7f417af4d811b3fae5bd069c94447f1fdc48a525b8d";
        let sig = Signature::from_hex(hex).expect("signature should parse");
        assert_eq!(sig.to_hex(), hex);
    }
}
