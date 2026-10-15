//! JWT revocation blacklist with timestamped entries and bounded growth.
//!
//! The blacklist stores `(jti, revoked_at, expires_at)` triples instead of
//! bare JTI strings so that entries can be pruned once both the revocation
//! and the original token expiration are in the past. Two backends are
//! provided:
//!
//! - [`InMemoryBlacklistStore`]: the default, kept entirely in process.
//! - [`RedisBlacklistStore`] (feature `cache`, in `redis_blacklist_store.rs`):
//!   persists revocations in Redis/Valkey so that revoked tokens remain
//!   rejected across process restarts, with a native TTL that bounds growth.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// A single blacklist entry with revocation and original-expiration
/// timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlacklistEntry {
    /// The token identifier (JTI) that was revoked.
    pub jti: String,
    /// Instant at which the token was revoked.
    pub revoked_at: DateTime<Utc>,
    /// Original token expiration instant (from the token's `exp` claim).
    pub expires_at: DateTime<Utc>,
}

impl BlacklistEntry {
    /// Creates a new entry.
    pub fn new(jti: String, revoked_at: DateTime<Utc>, expires_at: DateTime<Utc>) -> Self {
        Self {
            jti,
            revoked_at,
            expires_at,
        }
    }

    /// Returns `true` when both the revocation and the original expiration
    /// precede `instant` — the entry is fully expired and safe to prune.
    pub fn is_expired_before(&self, instant: DateTime<Utc>) -> bool {
        self.revoked_at < instant && self.expires_at < instant
    }
}

/// Storage backend for the token blacklist.
#[async_trait::async_trait]
pub trait BlacklistStore: Send + Sync {
    /// Stores a revocation entry.
    async fn insert(&self, entry: BlacklistEntry) -> Result<()>;

    /// Returns `true` when the given JTI is currently revoked.
    async fn contains(&self, jti: &str) -> bool;

    /// Removes every entry whose revocation and original expiration both
    /// precede `expired_before`. Returns the number of removed entries.
    async fn cleanup(&self, expired_before: DateTime<Utc>) -> usize;

    /// Returns the number of entries currently held (metric/observability).
    async fn len(&self) -> usize;

    /// Returns `true` when the store holds no entries.
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

/// In-memory blacklist backed by a `HashMap` guarded by an async `RwLock`.
#[derive(Debug, Default)]
pub struct InMemoryBlacklistStore {
    entries: Arc<RwLock<HashMap<String, BlacklistEntry>>>,
}

impl InMemoryBlacklistStore {
    /// Creates an empty in-memory blacklist.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl BlacklistStore for InMemoryBlacklistStore {
    async fn insert(&self, entry: BlacklistEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.jti.clone(), entry);
        Ok(())
    }

    async fn contains(&self, jti: &str) -> bool {
        let entries = self.entries.read().await;
        entries.contains_key(jti)
    }

    async fn cleanup(&self, expired_before: DateTime<Utc>) -> usize {
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|_, entry| !entry.is_expired_before(expired_before));
        let removed = before - entries.len();
        if removed > 0 {
            info!(
                "Token blacklist cleanup removed {removed} expired entries; {} remain",
                entries.len()
            );
        }
        removed
    }

    async fn len(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    fn entry(jti: &str, revoked_ago_secs: i64, expires_ago_secs: i64) -> BlacklistEntry {
        let revoked_at = now() - chrono::Duration::seconds(revoked_ago_secs);
        let expires_at = now() - chrono::Duration::seconds(expires_ago_secs);
        BlacklistEntry::new(jti.to_string(), revoked_at, expires_at)
    }

    #[tokio::test]
    async fn test_insert_and_contains() {
        let store = InMemoryBlacklistStore::new();
        store
            .insert(entry("jti-1", 10, 5))
            .await
            .expect("insert must succeed");
        assert!(store.contains("jti-1").await);
        assert!(!store.contains("jti-2").await);
    }

    #[tokio::test]
    async fn test_cleanup_removes_only_fully_expired() {
        let store = InMemoryBlacklistStore::new();
        // Revoked 100s ago and expired 100s ago → fully expired, prunable.
        store.insert(entry("expired", 100, 100)).await.expect("insert");
        // Revoked 100s ago but still valid for 5 more minutes → keep.
        store.insert(entry("revoked-recent", 100, -300)).await.expect("insert");
        // Revoked recently (still within its validity window) → keep.
        store.insert(entry("recent", -1, 300)).await.expect("insert");

        let removed = store.cleanup(now()).await;
        assert_eq!(removed, 1);
        assert!(!store.contains("expired").await);
        assert!(store.contains("revoked-recent").await);
        assert!(store.contains("recent").await);
        assert_eq!(store.len().await, 2);
    }

    #[tokio::test]
    async fn test_cleanup_empty_store() {
        let store = InMemoryBlacklistStore::new();
        assert_eq!(store.cleanup(now()).await, 0);
        assert_eq!(store.len().await, 0);
    }

    #[tokio::test]
    async fn test_entry_expired_semantics() {
        let cutoff = now();
        let fully_expired = entry("a", 10, 10);
        assert!(fully_expired.is_expired_before(cutoff));

        let revoked_only = entry("b", 10, -10);
        assert!(!revoked_only.is_expired_before(cutoff));

        let still_valid = entry("c", -1, 10);
        assert!(!still_valid.is_expired_before(cutoff));
    }
}
