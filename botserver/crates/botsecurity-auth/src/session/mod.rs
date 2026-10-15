pub(crate) use anyhow::{anyhow, Result};
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::{debug, info};
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub idle_timeout_minutes: i64,
    pub absolute_timeout_hours: i64,
    pub max_concurrent_sessions: usize,
    pub session_id_length: usize,
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub cookie_http_only: bool,
    pub cookie_same_site: SameSite,
    pub enable_device_tracking: bool,
    pub enable_ip_tracking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Expired,
    Revoked,
    Invalidated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct DeviceInfo {
    pub user_agent: Option<String>,
    pub device_type: Option<String>,
    pub os: Option<String>,
    pub browser: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: Uuid,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub absolute_expires_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub device_info: Option<DeviceInfo>,
    pub remember_me: bool,
    pub metadata: HashMap<String, String>,
}

pub trait SessionStore: Send + Sync {
    fn create(&self, session: Session) -> impl std::future::Future<Output = Result<()>> + Send;
    fn get(&self, session_id: &str) -> impl std::future::Future<Output = Result<Option<Session>>> + Send;
    fn update(&self, session: &Session) -> impl std::future::Future<Output = Result<()>> + Send;
    fn delete(&self, session_id: &str) -> impl std::future::Future<Output = Result<()>> + Send;
    fn get_user_sessions(&self, user_id: Uuid) -> impl std::future::Future<Output = Result<Vec<Session>>> + Send;
    fn delete_user_sessions(&self, user_id: Uuid) -> impl std::future::Future<Output = Result<usize>> + Send;
    fn cleanup_expired(&self) -> impl std::future::Future<Output = Result<usize>> + Send;
}

#[derive(Debug, Clone)]
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

pub struct SessionManager<S: SessionStore> {
    store: S,
    config: SessionConfig,
}

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn test_session_touch() {
        let config = SessionConfig {
            max_concurrent_sessions: 2,
            ..Default::default()
        };
        let user_id = Uuid::new_v4();
        let mut session = Session::new(user_id, &config);
        let original_expires = session.expires_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch(config.idle_timeout_minutes);

        assert!(session.expires_at > original_expires);
    }

    #[test]
    fn test_session_revoke() {
        let config = SessionConfig {
            ..Default::default()
        };
        let user_id = Uuid::new_v4();
        let mut session = Session::new(user_id, &config);

        assert!(session.is_valid());
        session.revoke();
        assert!(!session.is_valid());
        assert_eq!(session.status, SessionStatus::Revoked);
    }

    #[test]
    fn test_device_info_parsing() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0";
        let device = DeviceInfo::from_user_agent(ua);

        assert_eq!(device.os, Some("Windows".into()));
        assert_eq!(device.browser, Some("Chrome".into()));
        assert_eq!(device.device_type, Some("Desktop".into()));
    }

    #[test]
    fn test_device_info_mobile() {
        let ua = "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 Mobile Safari/537.36";
        let device = DeviceInfo::from_user_agent(ua);

        assert_eq!(device.os, Some("Android".into()));
        assert_eq!(device.device_type, Some("Mobile".into()));
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id(32);
        let id2 = generate_session_id(32);

        assert_eq!(id1.len(), 32);
        assert_eq!(id2.len(), 32);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_extract_session_from_cookie() {
        let cookie = "gb_session=abc123xyz; other=value";
        let session_id = extract_session_id_from_cookie(cookie, "gb_session");

        assert_eq!(session_id, Some("abc123xyz".into()));
    }

    #[test]
    fn test_session_config_defaults() {
        let config = SessionConfig::default();

        assert_eq!(config.idle_timeout_minutes, 30);
        assert_eq!(config.absolute_timeout_hours, 24);
        assert_eq!(config.max_concurrent_sessions, 5);
        assert!(config.cookie_secure);
        assert!(config.cookie_http_only);
    }

    #[test]
    fn test_remember_me_extends_session() {
        let config = SessionConfig::default();
        let user_id = Uuid::new_v4();
        let session_normal = Session::new(user_id, &config);
        let session_remember = Session::new(user_id, &config).with_remember_me(true);

        assert!(session_remember.absolute_expires_at > session_normal.absolute_expires_at);
    }

    #[test]
    fn test_same_site_as_str() {
        assert_eq!(SameSite::Strict.as_str(), "Strict");
        assert_eq!(SameSite::Lax.as_str(), "Lax");
        assert_eq!(SameSite::None.as_str(), "None");
    }

    #[tokio::test]
    async fn test_in_memory_store() {
        let store = InMemorySessionStore::new();
        let config = SessionConfig::default();
        let user_id = Uuid::new_v4();
        let session = Session::new(user_id, &config);
        let session_id = session.id.clone();

        store.create(session.clone()).await.expect("Create failed");

        let retrieved = store.get(&session_id).await.expect("Get failed");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|s| &s.id), Some(&session_id));

        store.delete(&session_id).await.expect("Delete failed");
        let deleted = store.get(&session_id).await.expect("Get failed");
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_create() {
        let store = InMemorySessionStore::new();
        let config = SessionConfig::default();
        let manager = SessionManager::new(store, config);
        let user_id = Uuid::new_v4();

        let session = manager
            .create_session(user_id, Some("127.0.0.1".into()), Some("Test Agent"), false)
            .await
            .expect("Create failed");

        assert_eq!(session.user_id, user_id);
        assert!(session.is_valid());
    }

    #[tokio::test]
    async fn test_session_manager_validate() {
        let store = InMemorySessionStore::new();
        let config = SessionConfig::default();
        let manager = SessionManager::new(store, config);
        let user_id = Uuid::new_v4();

        let session = manager
            .create_session(user_id, None, None, false)
            .await
            .expect("Create failed");

        let validated = manager
            .validate_session(&session.id)
            .await
            .expect("Validate failed");
        assert!(validated.is_some());

        let invalid = manager
            .validate_session("nonexistent")
            .await
            .expect("Validate failed");
        assert!(invalid.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_revoke() {
        let store = InMemorySessionStore::new();
        let config = SessionConfig::default();
        let manager = SessionManager::new(store, config);
        let user_id = Uuid::new_v4();

        let session = manager
            .create_session(user_id, None, None, false)
            .await
            .expect("Create failed");

        let revoked = manager
            .revoke_session(&session.id)
            .await
            .expect("Revoke failed");
        assert!(revoked);

        let validated = manager
            .validate_session(&session.id)
            .await
            .expect("Validate failed");
        assert!(validated.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_session_limit() {
        let store = InMemorySessionStore::new();
        let config = SessionConfig {
            max_concurrent_sessions: 2,
            ..Default::default()
        };
        let manager = SessionManager::new(store, config);
        let user_id = Uuid::new_v4();

        let s1 = manager
            .create_session(user_id, None, None, false)
            .await
            .expect("Create failed");
        let s2 = manager
            .create_session(user_id, None, None, false)
            .await
            .expect("Create failed");
        let s3 = manager
            .create_session(user_id, None, None, false)
            .await
            .expect("Create failed");

        let sessions = manager
            .get_user_sessions(user_id)
            .await
            .expect("Get sessions failed");
        assert_eq!(sessions.len(), 2);

        let ids: Vec<_> = sessions.iter().map(|s| s.id.clone()).collect();
        assert!(!ids.contains(&s1.id));
        assert!(ids.contains(&s2.id));
        assert!(ids.contains(&s3.id));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

pub use part6::*;
