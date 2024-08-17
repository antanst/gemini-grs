use std::time::{SystemTime, UNIX_EPOCH};

// returns the time since the epoch in milliseconds
pub fn now_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
