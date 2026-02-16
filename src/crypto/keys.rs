use std::fmt::{Display, Formatter};
use std::str::FromStr;

use secp256k1::ecdh;
use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::rand::thread_rng;
use secp256k1::{Message, PublicKey as SecpPublicKey, Secp256k1, SecretKey};

use crate::crypto::signature::Signature;
use crate::crypto::utils::{double_sha256, ripemd160, sha256, sha512};
use crate::error::{HiveError, Result};
use crate::serialization::serializer::transaction_digest;
use crate::types::{ChainId, SignedTransaction, Transaction};

const NETWORK_ID: u8 = 0x80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyRole {
    Owner,
    Active,
    Posting,
    Memo,
}

impl KeyRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Active => "active",
            Self::Posting => "posting",
            Self::Memo => "memo",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    pub(crate) key: Option<SecpPublicKey>,
    pub(crate) prefix: String,
}

impl PublicKey {
    pub fn from_string(value: &str) -> Result<Self> {
        if value.len() < 3 {
            return Err(HiveError::InvalidKey(
                "public key must include a 3-byte prefix".to_string(),
            ));
        }

        let prefix = &value[..3];
        let encoded = &value[3..];
        let decoded = bs58::decode(encoded)
            .into_vec()
            .map_err(|err| HiveError::InvalidKey(format!("invalid base58 public key: {err}")))?;

        if decoded.len() != 37 {
            return Err(HiveError::InvalidKey(format!(
                "public key payload must be 37 bytes, got {}",
                decoded.len()
            )));
        }

        let key_bytes: [u8; 33] = decoded[..33]
            .try_into()
            .map_err(|_| HiveError::InvalidKey("invalid public key payload".to_string()))?;
        let checksum = &decoded[33..37];
        let expected = &ripemd160(&key_bytes)[0..4];

        if checksum != expected {
            return Err(HiveError::InvalidKey(
                "public key checksum mismatch".to_string(),
            ));
        }

        if key_bytes == [0_u8; 33] {
            return Ok(Self {
                key: None,
                prefix: prefix.to_string(),
            });
        }

        let key = SecpPublicKey::from_slice(&key_bytes)
            .map_err(|err| HiveError::InvalidKey(format!("invalid public key bytes: {err}")))?;
        Ok(Self {
            key: Some(key),
            prefix: prefix.to_string(),
        })
    }

    pub fn from_bytes(bytes: [u8; 33], prefix: impl Into<String>) -> Result<Self> {
        if bytes == [0_u8; 33] {
            return Ok(Self {
                key: None,
                prefix: prefix.into(),
            });
        }
        let key = SecpPublicKey::from_slice(&bytes)
            .map_err(|err| HiveError::InvalidKey(format!("invalid public key bytes: {err}")))?;
        Ok(Self {
            key: Some(key),
            prefix: prefix.into(),
        })
    }

    pub(crate) fn from_secp256k1(key: SecpPublicKey, prefix: impl Into<String>) -> Self {
        Self {
            key: Some(key),
            prefix: prefix.into(),
        }
    }

    pub fn to_string_with_prefix(&self, prefix: &str) -> String {
        let key_bytes = self.compressed_bytes();
        let checksum = ripemd160(&key_bytes);
        let mut data = Vec::with_capacity(37);
        data.extend_from_slice(&key_bytes);
        data.extend_from_slice(&checksum[..4]);
        format!("{prefix}{}", bs58::encode(data).into_string())
    }

    pub fn compressed_bytes(&self) -> [u8; 33] {
        match self.key {
            Some(key) => key.serialize(),
            None => [0_u8; 33],
        }
    }

    pub fn is_null(&self) -> bool {
        self.key.is_none()
    }

    pub fn prefix(&self) -> &str {
        self.prefix.as_str()
    }

    pub fn verify(&self, digest: &[u8; 32], signature: &Signature) -> bool {
        let Some(public_key) = &self.key else {
            return false;
        };

        let msg = Message::from_digest_slice(digest);
        let sig = secp256k1::ecdsa::Signature::from_compact(&signature.compact_bytes());
        match (msg, sig) {
            (Ok(msg), Ok(sig)) => {
                let secp = Secp256k1::verification_only();
                secp.verify_ecdsa(&msg, &sig, public_key).is_ok()
            }
            _ => false,
        }
    }
}

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_with_prefix(&self.prefix))
    }
}

impl FromStr for PublicKey {
    type Err = HiveError;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateKey {
    pub(crate) secret: SecretKey,
}

impl PrivateKey {
    pub fn from_wif(wif: &str) -> Result<Self> {
        let decoded = bs58::decode(wif)
            .into_vec()
            .map_err(|err| HiveError::InvalidKey(format!("invalid base58 wif: {err}")))?;

        if decoded.len() != 37 {
            return Err(HiveError::InvalidKey(format!(
                "wif payload must be 37 bytes, got {}",
                decoded.len()
            )));
        }

        if decoded[0] != NETWORK_ID {
            return Err(HiveError::InvalidKey(
                "private key network id mismatch".to_string(),
            ));
        }

        let payload = &decoded[..33];
        let checksum = &decoded[33..37];
        let expected = &double_sha256(payload)[..4];
        if checksum != expected {
            return Err(HiveError::InvalidKey(
                "private key checksum mismatch".to_string(),
            ));
        }

        let key_bytes: [u8; 32] = payload[1..33]
            .try_into()
            .map_err(|_| HiveError::InvalidKey("invalid private key bytes".to_string()))?;
        Self::from_bytes(key_bytes)
    }

    pub fn from_seed(seed: &str) -> Result<Self> {
        Self::from_bytes(sha256(seed.as_bytes()))
    }

    pub fn from_login(username: &str, password: &str, role: KeyRole) -> Result<Self> {
        let seed = format!("{username}{}{password}", role.as_str());
        Self::from_seed(&seed)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self> {
        let secret = SecretKey::from_slice(&bytes)
            .map_err(|err| HiveError::InvalidKey(format!("invalid private key bytes: {err}")))?;
        Ok(Self { secret })
    }

    pub fn generate() -> Self {
        let mut rng = thread_rng();
        let secret = SecretKey::new(&mut rng);
        Self { secret }
    }

    pub fn to_wif(&self) -> String {
        let mut payload = [0_u8; 33];
        payload[0] = NETWORK_ID;
        payload[1..].copy_from_slice(&self.secret.secret_bytes());
        let checksum = double_sha256(&payload);
        let mut full = Vec::with_capacity(37);
        full.extend_from_slice(&payload);
        full.extend_from_slice(&checksum[..4]);
        bs58::encode(full).into_string()
    }

    pub fn public_key(&self) -> PublicKey {
        let secp = Secp256k1::new();
        let key = SecpPublicKey::from_secret_key(&secp, &self.secret);
        PublicKey::from_secp256k1(key, "STM")
    }

    pub fn sign(&self, digest: &[u8; 32]) -> Result<Signature> {
        let secp = Secp256k1::new();
        let msg = Message::from_digest_slice(digest)
            .map_err(|err| HiveError::Signing(format!("invalid digest: {err}")))?;

        let mut attempts = 0_u16;
        loop {
            attempts = attempts.saturating_add(1);
            let nonce_seed = sha256(&[digest.as_slice(), &[(attempts as u8)]].concat());
            let recoverable =
                secp.sign_ecdsa_recoverable_with_noncedata(&msg, &self.secret, &nonce_seed);
            let (recovery_id, compact) = recoverable.serialize_compact();
            if Signature::is_canonical_compact(&compact) {
                return Signature::from_compact(compact, recovery_id.to_i32() as u8);
            }

            if attempts == u16::MAX {
                return Err(HiveError::Signing(
                    "unable to produce canonical signature".to_string(),
                ));
            }
        }
    }

    pub fn get_shared_secret(&self, public_key: &PublicKey) -> [u8; 64] {
        let Some(key) = &public_key.key else {
            return [0_u8; 64];
        };

        let point = ecdh::shared_secret_point(key, &self.secret);
        let x_coord = &point[..32];
        sha512(x_coord)
    }

    pub fn secret_bytes(&self) -> [u8; 32] {
        self.secret.secret_bytes()
    }
}

impl Display for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_wif())
    }
}

impl FromStr for PrivateKey {
    type Err = HiveError;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_wif(s)
    }
}

impl TryFrom<&str> for PrivateKey {
    type Error = HiveError;

    fn try_from(value: &str) -> Result<Self> {
        Self::from_wif(value)
    }
}

impl TryFrom<String> for PrivateKey {
    type Error = HiveError;

    fn try_from(value: String) -> Result<Self> {
        Self::from_wif(&value)
    }
}

pub(crate) fn recoverable_from_signature(signature: &Signature) -> Result<RecoverableSignature> {
    let rec_id = RecoveryId::from_i32(signature.recovery_id() as i32)
        .map_err(|err| HiveError::Signing(format!("invalid recovery id: {err}")))?;
    RecoverableSignature::from_compact(&signature.compact_bytes(), rec_id)
        .map_err(|err| HiveError::Signing(format!("invalid compact signature: {err}")))
}

pub fn sign_transaction(
    transaction: &Transaction,
    keys: &[&PrivateKey],
    chain_id: &ChainId,
) -> Result<SignedTransaction> {
    let digest = transaction_digest(transaction, chain_id)?;
    let signatures = keys
        .iter()
        .map(|key| key.sign(&digest).map(|sig| sig.to_hex()))
        .collect::<Result<Vec<_>>>()?;

    Ok(SignedTransaction {
        ref_block_num: transaction.ref_block_num,
        ref_block_prefix: transaction.ref_block_prefix,
        expiration: transaction.expiration.clone(),
        operations: transaction.operations.clone(),
        extensions: transaction.extensions.clone(),
        signatures,
    })
}

#[cfg(test)]
mod tests {
    use crate::crypto::keys::{sign_transaction, KeyRole, PrivateKey, PublicKey};
    use crate::types::{ChainId, Operation, Transaction, VoteOperation};

    #[test]
    fn from_login_matches_dhive_vector() {
        let key = PrivateKey::from_login("foo", "barman", KeyRole::Active).expect("valid key");
        assert_eq!(
            key.public_key().to_string(),
            "STM87F7tN56tAUL2C6J9Gzi9HzgNpZdi6M2cLQo7TjDU5v178QsYA"
        );
    }

    #[test]
    fn wif_round_trip() {
        let key = PrivateKey::generate();
        let wif = key.to_wif();
        let parsed = PrivateKey::from_wif(&wif).expect("wif should parse");
        assert_eq!(parsed.secret_bytes(), key.secret_bytes());
    }

    #[test]
    fn known_wif_to_public_key() {
        let key = PrivateKey::from_wif("5KG4sr3rMH1QuduYj79p36h7PrEeZakHEPjB9NkLWqgw19DDieL")
            .expect("wif should parse");
        assert_eq!(
            key.public_key().to_string(),
            "STM87F7tN56tAUL2C6J9Gzi9HzgNpZdi6M2cLQo7TjDU5v178QsYA"
        );
    }

    #[test]
    fn public_key_round_trip() {
        let key = PublicKey::from_string("STM87F7tN56tAUL2C6J9Gzi9HzgNpZdi6M2cLQo7TjDU5v178QsYA")
            .expect("public key should parse");
        assert_eq!(
            key.to_string(),
            "STM87F7tN56tAUL2C6J9Gzi9HzgNpZdi6M2cLQo7TjDU5v178QsYA"
        );
    }

    #[test]
    fn detects_null_public_key() {
        let key = PublicKey::from_string("STM1111111111111111111111111111111114T1Anm")
            .expect("null public key should parse");
        assert!(key.is_null());
        assert_eq!(key.compressed_bytes(), [0_u8; 33]);
    }

    #[test]
    fn sign_transaction_matches_dhive_vector() {
        let key = PrivateKey::from_wif("5KG4sr3rMH1QuduYj79p36h7PrEeZakHEPjB9NkLWqgw19DDieL")
            .expect("wif should parse");
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
        let signed = sign_transaction(&tx, &[&key], &chain_id).expect("transaction should sign");
        assert_eq!(
            signed.signatures[0],
            "1f037a09c1110a8bd8757ad3081a11456d241feedd4366723bb9f9046cc6a1b21b26bf4b8372546bc2446c7498ff5742dce0143ff1fe13591eb8dd88b9a7fef2f2"
        );
    }
}
