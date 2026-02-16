use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static NONCE_ENTROPY: OnceLock<AtomicU32> = OnceLock::new();

pub fn unique_nonce() -> u64 {
    let entropy = NONCE_ENTROPY.get_or_init(|| {
        let seed = rand::random::<u16>() as u32;
        AtomicU32::new(seed)
    });

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default();

    let low = (entropy.fetch_add(1, Ordering::Relaxed) + 1) % 0xFFFF;
    (now_ms << 16) | (low as u64)
}

#[cfg(test)]
mod tests {
    use crate::utils::unique_nonce;

    #[test]
    fn nonces_are_unique_for_sequential_calls() {
        let first = unique_nonce();
        let second = unique_nonce();
        assert_ne!(first, second);
    }
}
