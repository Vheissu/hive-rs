use secp256k1::rand::thread_rng;
use secp256k1::{PublicKey as SecpPublicKey, Secp256k1, SecretKey};

use crate::crypto::utils::sha256;
use crate::error::{HiveError, Result};

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

#[derive(Debug, Clone)]
pub struct PublicKey {
    pub(crate) key: SecpPublicKey,
    pub(crate) prefix: String,
}

impl PublicKey {
    pub fn from_secp256k1(key: SecpPublicKey, prefix: impl Into<String>) -> Self {
        Self {
            key,
            prefix: prefix.into(),
        }
    }

    pub fn compressed_bytes(&self) -> [u8; 33] {
        self.key.serialize()
    }

    pub fn prefix(&self) -> &str {
        self.prefix.as_str()
    }
}

#[derive(Debug, Clone)]
pub struct PrivateKey {
    pub(crate) secret: SecretKey,
}

impl PrivateKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self> {
        let secret = SecretKey::from_slice(&bytes)
            .map_err(|err| HiveError::InvalidKey(format!("invalid private key bytes: {err}")))?;
        Ok(Self { secret })
    }

    pub fn from_seed(seed: &str) -> Result<Self> {
        Self::from_bytes(sha256(seed.as_bytes()))
    }

    pub fn from_login(username: &str, password: &str, role: KeyRole) -> Result<Self> {
        let seed = format!("{}{}{}", username, role.as_str(), password);
        Self::from_seed(&seed)
    }

    pub fn generate() -> Self {
        let mut rng = thread_rng();
        let secret = SecretKey::new(&mut rng);
        Self { secret }
    }

    pub fn public_key(&self) -> PublicKey {
        let secp = Secp256k1::new();
        let key = SecpPublicKey::from_secret_key(&secp, &self.secret);
        PublicKey::from_secp256k1(key, "STM")
    }
}
