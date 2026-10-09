//! Anonymous session data types.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousSession {
    pub id: Uuid,
    pub fingerprint: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub message_count: u32,
    pub bot_id: Option<Uuid>,
    pub metadata: HashMap<String, String>,
    pub upgraded_to_user_id: Option<Uuid>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousSessionConfig {
    pub session_ttl_minutes: i64,
    pub max_messages_per_session: u32,
    pub max_sessions_per_ip: u32,
    pub require_fingerprint: bool,
    pub allow_session_upgrade: bool,
    pub preserve_history_on_upgrade: bool,
}

impl Default for AnonymousSessionConfig {
    fn default() -> Self {
        Self {
            session_ttl_minutes: 60,
            max_messages_per_session: 20,
            max_sessions_per_ip: 5,
            require_fingerprint: false,
            allow_session_upgrade: true,
            preserve_history_on_upgrade: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpgradeResult {
    pub success: bool,
    pub anonymous_session_id: Uuid,
    pub user_id: Uuid,
    pub messages_migrated: u32,
    pub upgraded_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub upgraded_sessions: usize,
    pub total_messages: usize,
    pub config: AnonymousSessionConfig,
}

#[derive(Debug, Clone)]
pub enum AnonymousSessionError {
    SessionNotFound,
    SessionExpired,
    MessageLimitReached,
    TooManySessions,
    FingerprintRequired,
    UpgradeNotAllowed,
    AlreadyUpgraded,
    InvalidSession,
}

impl std::fmt::Display for AnonymousSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionNotFound => write!(f, "Session not found"),
            Self::SessionExpired => write!(f, "Session has expired"),
            Self::MessageLimitReached => {
                write!(f, "Message limit reached for anonymous session")
            }
            Self::TooManySessions => write!(f, "Too many sessions from this IP address"),
            Self::FingerprintRequired => write!(f, "Browser fingerprint is required"),
            Self::UpgradeNotAllowed => write!(f, "Session upgrade is not allowed"),
            Self::AlreadyUpgraded => write!(f, "Session has already been upgraded"),
            Self::InvalidSession => write!(f, "Invalid session"),
        }
    }
}

impl std::error::Error for AnonymousSessionError {}

