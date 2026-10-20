use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl Session {
    #[must_use]
    pub fn new(user_id: Uuid, bot_id: Uuid, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            bot_id,
            title: title.into(),
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    #[must_use]
    pub const fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Utc::now() > exp)
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.is_expired()
    }

    #[must_use]
    pub fn remaining_time(&self) -> Option<chrono::Duration> {
        self.expires_at.map(|exp| exp - Utc::now())
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::user_sessions;

// Plain DTO: this type is never loaded through Diesel (the channel crates
// own their own row types), so it must not be coupled to the table shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub branch_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub context_data: serde_json::Value,
    pub current_tool: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserSession {
    #[must_use]
    pub fn new(user_id: Uuid, bot_id: Uuid, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            branch_id: Uuid::nil(),
            bot_id,
            title: title.into(),
            context_data: serde_json::json!({}),
            current_tool: None,
            created_at: now,
            updated_at: now,
        }
    }
}
