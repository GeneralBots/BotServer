use super::*;

impl<V> CacheEntry<V> {
    pub fn new(value: V, ttl_seconds: u64, size_bytes: usize) -> Self {
        let now = Utc::now();
        Self {
            value,
            created_at: now,
            expires_at: now + Duration::seconds(ttl_seconds as i64),
            access_count: 0,
            last_accessed: now,
            size_bytes,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }
}
