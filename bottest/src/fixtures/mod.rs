pub mod data;
pub mod scripts;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            email: "user@example.com".to_string(),
            name: "Test User".to_string(),
            role: Role::User,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Role {
    Admin,
    Attendant,
    #[default]
    User,
    Guest,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: Uuid,
    pub external_id: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub channel: Channel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl Default for Customer {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            external_id: format!("ext_{}", Uuid::new_v4()),
            phone: Some("+15551234567".to_string()),
            email: None,
            name: Some("Test Customer".to_string()),
            channel: Channel::WhatsApp,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Channel {
    #[default]
    WhatsApp,
    Teams,
    Web,
    Sms,
    Email,
    Api,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub kb_enabled: bool,
    pub llm_enabled: bool,
    pub llm_model: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: HashMap<String, serde_json::Value>,
}

impl Default for Bot {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "test-bot".to_string(),
            description: Some("Test bot for automated testing".to_string()),
            kb_enabled: false,
            llm_enabled: true,
            llm_model: Some("gpt-4".to_string()),
            active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub customer_id: Uuid,
    pub channel: Channel,
    pub state: SessionState,
    pub context: HashMap<String, serde_json::Value>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            bot_id: Uuid::new_v4(),
            customer_id: Uuid::new_v4(),
            channel: Channel::WhatsApp,
            state: SessionState::Active,
            context: HashMap::new(),
            started_at: Utc::now(),
            updated_at: Utc::now(),
            ended_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SessionState {
    #[default]
    Active,
    Waiting,
    Transferred,
    Ended,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub direction: MessageDirection,
    pub content: String,
    pub content_type: ContentType,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            direction: MessageDirection::Incoming,
            content: "Hello".to_string(),
            content_type: ContentType::Text,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ContentType {
    #[default]
    Text,
    Image,
    Audio,
    Video,
    Document,
    Location,
    Contact,
    Interactive,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub session_id: Uuid,
    pub priority: Priority,
    pub status: QueueStatus,
    pub entered_at: DateTime<Utc>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub attendant_id: Option<Uuid>,
}

impl Default for QueueEntry {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            customer_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            priority: Priority::Normal,
            status: QueueStatus::Waiting,
            entered_at: Utc::now(),
            assigned_at: None,
            attendant_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Priority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Urgent = 3,
}

mod part2;
pub use part2::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_user() {
        let user = admin_user();
        assert_eq!(user.role, Role::Admin);
        assert_eq!(user.email, "admin@test.com");
    }

    #[test]
    fn test_customer_factory() {
        let c = customer("+15559876543");
        assert_eq!(c.phone, Some("+15559876543".to_string()));
        assert_eq!(c.channel, Channel::WhatsApp);
    }

    #[test]
    fn test_bot_with_kb() {
        let bot = bot_with_kb("kb-bot");
        assert!(bot.kb_enabled);
        assert!(bot.llm_enabled);
    }

    #[test]
    fn test_session_for() {
        let bot = basic_bot("test");
        let customer = customer("+15551234567");
        let session = session_for(&bot, &customer);

        assert_eq!(session.bot_id, bot.id);
        assert_eq!(session.customer_id, customer.id);
        assert_eq!(session.channel, customer.channel);
    }

    #[test]
    fn test_message_factories() {
        let incoming = incoming_message("Hello");
        assert_eq!(incoming.direction, MessageDirection::Incoming);
        assert_eq!(incoming.content, "Hello");

        let outgoing = outgoing_message("Hi there!");
        assert_eq!(outgoing.direction, MessageDirection::Outgoing);
        assert_eq!(outgoing.content, "Hi there!");
    }

    #[test]
    fn test_queue_entry_priority() {
        let normal = QueueEntry::default();
        let high = high_priority_queue_entry();
        let urgent = urgent_queue_entry();

        assert!(urgent.priority > high.priority);
        assert!(high.priority > normal.priority);
    }

    #[test]
    fn test_default_implementations() {
        let _user = User::default();
        let _customer = Customer::default();
        let _bot = Bot::default();
        let _session = Session::default();
        let _message = Message::default();
        let _queue = QueueEntry::default();
    }
}
