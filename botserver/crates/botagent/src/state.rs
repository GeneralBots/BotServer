//! Shared state primitives: DB pool alias and the in-memory rate limiter.

pub use botlib::db_pool::DbPool;

/// Simple token bucket: each key holds the last refill instant and the
/// remaining tokens. Capacity equals `per_sec`, refilled linearly.
#[derive(Default)]
pub struct RateLimiter {
    buckets: std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, f32)>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when the call is allowed and consumes one token.
    pub fn check(&self, key: &str, per_sec: f32) -> bool {
        let now = std::time::Instant::now();
        let mut buckets = match self.buckets.lock() {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("rate limiter lock poisoned: {e}");
                return false;
            }
        };
        let entry = buckets
            .entry(key.to_string())
            .or_insert_with(|| (now, per_sec));
        let elapsed = now.duration_since(entry.0).as_secs_f32();
        entry.0 = now;
        let capacity = if per_sec <= 0.0 { 1.0 } else { per_sec };
        let refilled = (entry.1 + elapsed * per_sec).min(capacity);
        if refilled >= 1.0 {
            entry.1 = refilled - 1.0;
            true
        } else {
            entry.1 = refilled;
            false
        }
    }
}
