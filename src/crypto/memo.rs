use aes::Aes256;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};

use crate::crypto::keys::{PrivateKey, PublicKey};
use crate::crypto::utils::{sha256, sha512};
use crate::error::{HiveError, Result};
use crate::serialization::types::{
    read_string, write_string, write_u32, write_u64, write_varint32,
};
use crate::utils::unique_nonce;

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

#[derive(Debug, Clone)]
struct EncryptedMemoPayload {
    from: PublicKey,
    to: PublicKey,
    nonce: u64,
    check: u32,
    encrypted: Vec<u8>,
}

pub fn encode(
    message: &str,
    sender_private: &PrivateKey,
    receiver_public: &PublicKey,
) -> Result<String> {
    let nonce = unique_nonce();
    encode_with_nonce(message, sender_private, receiver_public, nonce)
}

pub fn encode_with_nonce(
    message: &str,
    sender_private: &PrivateKey,
    receiver_public: &PublicKey,
    nonce: u64,
) -> Result<String> {
    if !message.starts_with('#') {
        return Ok(message.to_string());
    }

    let plaintext = &message[1..];
    let mut plain_bytes = Vec::new();
    write_string(&mut plain_bytes, plaintext);

    let (key, iv, check) = derive_aes_params(sender_private, receiver_public, nonce);

    let mut encrypt_buf = plain_bytes.clone();
    let block_size = 16;
    let msg_len = encrypt_buf.len();
    let pad_len = block_size - (msg_len % block_size);
    encrypt_buf.resize(msg_len + pad_len, 0);
    let encrypted = Aes256CbcEnc::new(&key.into(), &iv.into())
        .encrypt_padded_mut::<Pkcs7>(&mut encrypt_buf, msg_len)
        .map_err(|err| HiveError::Signing(format!("memo encrypt failed: {err}")))?
        .to_vec();

    let payload = EncryptedMemoPayload {
        from: sender_private.public_key(),
        to: receiver_public.clone(),
        nonce,
        check,
        encrypted,
    };

    let serialized = serialize_encrypted_memo(&payload);
    Ok(format!("#{}", bs58::encode(serialized).into_string()))
}

pub fn decode(encoded: &str, receiver_private: &PrivateKey) -> Result<String> {
    if !encoded.starts_with('#') {
        return Ok(encoded.to_string());
    }

    let raw = bs58::decode(&encoded[1..])
        .into_vec()
        .map_err(|err| HiveError::Signing(format!("invalid base58 memo: {err}")))?;
    let payload = deserialize_encrypted_memo(&raw)?;

    let my_public = receiver_private.public_key().to_string();
    let from = payload.from.to_string();
    let to = payload.to.to_string();
    let other_public = if my_public == from {
        payload.to
    } else if my_public == to {
        payload.from
    } else {
        // Fallback to sender key for compatibility with externally encoded memos.
        payload.from
    };

    let (key, iv, check) = derive_aes_params(receiver_private, &other_public, payload.nonce);
    if check != payload.check {
        return Err(HiveError::Signing("Invalid key".to_string()));
    }

    let mut decrypt_buf = payload.encrypted.clone();
    let decrypted = Aes256CbcDec::new(&key.into(), &iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut decrypt_buf)
        .map_err(|err| HiveError::Signing(format!("memo decrypt failed: {err}")))?
        .to_vec();

    // dhive first tries VString, then raw UTF-8 fallback.
    let mut cursor = decrypted.as_slice();
    if let Ok(text) = read_string(&mut cursor) {
        if cursor.is_empty() {
            return Ok(format!("#{text}"));
        }
    }

    let text = String::from_utf8(decrypted)
        .map_err(|err| HiveError::Signing(format!("memo plaintext is not valid UTF-8: {err}")))?;
    Ok(format!("#{text}"))
}

fn derive_aes_params(
    private_key: &PrivateKey,
    public_key: &PublicKey,
    nonce: u64,
) -> ([u8; 32], [u8; 16], u32) {
    let shared = private_key.get_shared_secret(public_key);
    let mut seed = Vec::with_capacity(8 + shared.len());
    write_u64(&mut seed, nonce);
    seed.extend_from_slice(&shared);
    let encryption_key = sha512(&seed);

    let mut key = [0_u8; 32];
    key.copy_from_slice(&encryption_key[..32]);
    let mut iv = [0_u8; 16];
    iv.copy_from_slice(&encryption_key[32..48]);
    let check_hash = sha256(&encryption_key);
    let check = u32::from_le_bytes(check_hash[0..4].try_into().expect("slice length fixed"));
    (key, iv, check)
}

fn serialize_encrypted_memo(payload: &EncryptedMemoPayload) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&payload.from.compressed_bytes());
    buf.extend_from_slice(&payload.to.compressed_bytes());
    write_u64(&mut buf, payload.nonce);
    write_u32(&mut buf, payload.check);
    write_varint32(&mut buf, payload.encrypted.len() as u32);
    buf.extend_from_slice(&payload.encrypted);
    buf
}

fn deserialize_encrypted_memo(input: &[u8]) -> Result<EncryptedMemoPayload> {
    let mut cursor = input;
    let from = read_public_key(&mut cursor)?;
    let to = read_public_key(&mut cursor)?;
    let nonce = read_u64(&mut cursor)?;
    let check = read_u32(&mut cursor)?;
    let encrypted = read_variable_binary(&mut cursor)?;

    Ok(EncryptedMemoPayload {
        from,
        to,
        nonce,
        check,
        encrypted,
    })
}

fn read_public_key(cursor: &mut &[u8]) -> Result<PublicKey> {
    if cursor.len() < 33 {
        return Err(HiveError::Serialization(
            "encrypted memo payload is truncated".to_string(),
        ));
    }
    let bytes: [u8; 33] = cursor[..33]
        .try_into()
        .map_err(|_| HiveError::Serialization("invalid public key bytes".to_string()))?;
    *cursor = &cursor[33..];
    PublicKey::from_bytes(bytes, "STM")
}

fn read_u32(cursor: &mut &[u8]) -> Result<u32> {
    if cursor.len() < 4 {
        return Err(HiveError::Serialization(
            "encrypted memo payload missing u32".to_string(),
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

fn read_u64(cursor: &mut &[u8]) -> Result<u64> {
    if cursor.len() < 8 {
        return Err(HiveError::Serialization(
            "encrypted memo payload missing u64".to_string(),
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

fn read_varint32(cursor: &mut &[u8]) -> Result<u32> {
    let mut result = 0_u32;
    let mut shift = 0_u32;
    let mut index = 0_usize;

    while index < cursor.len() {
        let byte = cursor[index];
        result |= ((byte & 0x7F) as u32) << shift;
        index += 1;
        if byte & 0x80 == 0 {
            *cursor = &cursor[index..];
            return Ok(result);
        }
        shift += 7;
        if shift > 28 {
            return Err(HiveError::Serialization(
                "varint32 is too large".to_string(),
            ));
        }
    }

    Err(HiveError::Serialization(
        "unexpected EOF while reading varint32".to_string(),
    ))
}

fn read_variable_binary(cursor: &mut &[u8]) -> Result<Vec<u8>> {
    let len = read_varint32(cursor)? as usize;
    if cursor.len() < len {
        return Err(HiveError::Serialization(
            "encrypted memo payload has invalid binary length".to_string(),
        ));
    }
    let data = cursor[..len].to_vec();
    *cursor = &cursor[len..];
    Ok(data)
}

#[cfg(test)]
mod tests {
    use crate::crypto::keys::{PrivateKey, PublicKey};
    use crate::crypto::memo::{decode, encode_with_nonce};

    #[test]
    fn encrypt_and_decrypt_round_trip() {
        let sender = PrivateKey::from_wif("5JdeC9P7Pbd1uGdFVEsJ41EkEnADbbHGq6p1BwFxm6txNBsQnsw")
            .expect("valid sender key");
        let recipient =
            PublicKey::from_string("STM8m5UgaFAAYQRuaNejYdS8FVLVp9Ss3K1qAVk5de6F8s3HnVbvA")
                .expect("valid public key");

        let encoded = encode_with_nonce("#memo爱", &sender, &recipient, 1_234_567_890)
            .expect("memo encode should succeed");
        let decoded = decode(&encoded, &sender).expect("memo decode should succeed");
        assert_eq!(decoded, "#memo爱");
    }

    #[test]
    fn matches_dhive_encryption_vector() {
        let sender = PrivateKey::from_wif("5JdeC9P7Pbd1uGdFVEsJ41EkEnADbbHGq6p1BwFxm6txNBsQnsw")
            .expect("valid sender key");
        let recipient =
            PublicKey::from_string("STM8m5UgaFAAYQRuaNejYdS8FVLVp9Ss3K1qAVk5de6F8s3HnVbvA")
                .expect("valid public key");

        let encoded = encode_with_nonce("#memo爱", &sender, &recipient, 1_234_567_890)
            .expect("memo encode should succeed");
        assert_eq!(
            encoded,
            "#K55WaPFbgNW8w8UiPzFGRejmMLZH3CA6guETaVLS7fUGgYhSwWTXjQ26ozhA6zFtG339Tsjw5AXqce8v4HCsYZ9kG3mStgR9ixN9KWPUpFDFgST38EoeWVncvfsCPFseg"
        );
    }

    #[test]
    fn rejects_invalid_checksum() {
        let receiver = PrivateKey::from_wif("5JdeC9P7Pbd1uGdFVEsJ41EkEnADbbHGq6p1BwFxm6txNBsQnsw")
            .expect("valid key");
        let bad = "#K55WaPFbgNW8w8UiPzFGRejmMLZH3CA6guETaVLS7fUGgYhSwWTXjQ26ozhA6zFtG339Tsjw5AXqce8v4HCsYZ9kG3mStgR9ixN9KWPUpFDFgST38EoeWVncvfsCPFse1";
        assert!(decode(bad, &receiver).is_err());
    }

    #[test]
    fn leaves_plaintext_memo_unchanged() {
        let receiver = PrivateKey::from_wif("5JdeC9P7Pbd1uGdFVEsJ41EkEnADbbHGq6p1BwFxm6txNBsQnsw")
            .expect("valid key");
        assert_eq!(
            decode("plain memo", &receiver).expect("decode should pass through"),
            "plain memo"
        );
    }
}
