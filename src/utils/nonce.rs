use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn unique_nonce() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();
    let count = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    (millis << 16) ^ (count & 0xFFFF)
}
