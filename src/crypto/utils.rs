use ripemd::{Digest as RipemdDigest, Ripemd160};
use sha2::{Sha256, Sha512};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn double_sha256(data: &[u8]) -> [u8; 32] {
    sha256(&sha256(data))
}

pub fn ripemd160(data: &[u8]) -> [u8; 20] {
    let mut hasher = Ripemd160::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn sha512(data: &[u8]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{double_sha256, ripemd160, sha256, sha512};

    #[test]
    fn known_hash_vectors() {
        assert_eq!(
            hex::encode(sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex::encode(double_sha256(b"abc")),
            "4f8b42c22dd3729b519ba6f68d2da7cc5b2d606d05daed5ad5128cc03e6c6358"
        );
        assert_eq!(
            hex::encode(ripemd160(b"abc")),
            "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc"
        );
        assert_eq!(hex::encode(sha512(b"abc")), "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
    }
}
