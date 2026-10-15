//! Split from `mod.rs` — 260 lines.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum QueueStatus {
    #[default]
    Waiting,
    Assigned,
    InProgress,
    Completed,
    Cancelled,
}

#[must_use]
pub fn admin_user() -> User {
    User {
        email: "admin@test.com".to_string(),
        name: "Test Admin".to_string(),
        role: Role::Admin,
        ..Default::default()
    }
}

#[must_use]
pub fn attendant_user() -> User {
    User {
        email: "attendant@test.com".to_string(),
        name: "Test Attendant".to_string(),
        role: Role::Attendant,
        ..Default::default()
    }
}

#[must_use]
pub fn regular_user() -> User {
    User {
        email: "user@test.com".to_string(),
        name: "Test User".to_string(),
        role: Role::User,
        ..Default::default()
    }
}

#[must_use]
pub fn user_with_email(email: &str) -> User {
    User {
        email: email.to_string(),
        name: email.split('@').next().unwrap_or("User").to_string(),
        ..Default::default()
    }
}

#[must_use]
pub fn customer(phone: &str) -> Customer {
    Customer {
        phone: Some(phone.to_string()),
        channel: Channel::WhatsApp,
        ..Default::default()
    }
}

#[must_use]
pub fn customer_on_channel(channel: Channel) -> Customer {
    Customer {
        channel,
        ..Default::default()
    }
}

#[must_use]
pub fn teams_customer() -> Customer {
    Customer {
        channel: Channel::Teams,
        external_id: format!("teams_{}", Uuid::new_v4()),
        ..Default::default()
    }
}

#[must_use]
pub fn web_customer() -> Customer {
    Customer {
        channel: Channel::Web,
        external_id: format!("web_{}", Uuid::new_v4()),
        ..Default::default()
    }
}

#[must_use]
pub fn basic_bot(name: &str) -> Bot {
    Bot {
        name: name.to_string(),
        kb_enabled: false,
        llm_enabled: true,
        ..Default::default()
    }
}

#[must_use]
pub fn bot_with_kb(name: &str) -> Bot {
    Bot {
        name: name.to_string(),
        kb_enabled: true,
        llm_enabled: true,
        ..Default::default()
    }
}

#[must_use]
pub fn rule_based_bot(name: &str) -> Bot {
    Bot {
        name: name.to_string(),
        kb_enabled: false,
        llm_enabled: false,
        llm_model: None,
        ..Default::default()
    }
}

#[must_use]
pub fn session_for(bot: &Bot, customer: &Customer) -> Session {
    Session {
        bot_id: bot.id,
        customer_id: customer.id,
        channel: customer.channel,
        ..Default::default()
    }
}

#[must_use]
pub fn active_session() -> Session {
    Session {
        state: SessionState::Active,
        ..Default::default()
    }
}

#[must_use]
pub fn incoming_message(content: &str) -> Message {
    Message {
        direction: MessageDirection::Incoming,
        content: content.to_string(),
        ..Default::default()
    }
}

#[must_use]
pub fn outgoing_message(content: &str) -> Message {
    Message {
        direction: MessageDirection::Outgoing,
        content: content.to_string(),
        ..Default::default()
    }
}

#[must_use]
pub fn message_in_session(
    session: &Session,
    content: &str,
    direction: MessageDirection,
) -> Message {
    Message {
        session_id: session.id,
        direction,
        content: content.to_string(),
        ..Default::default()
    }
}

#[must_use]
pub fn queue_entry_for(customer: &Customer, session: &Session) -> QueueEntry {
    QueueEntry {
        customer_id: customer.id,
        session_id: session.id,
        ..Default::default()
    }
}

#[must_use]
pub fn high_priority_queue_entry() -> QueueEntry {
    QueueEntry {
        priority: Priority::High,
        ..Default::default()
    }
}

#[must_use]
pub fn urgent_queue_entry() -> QueueEntry {
    QueueEntry {
        priority: Priority::Urgent,
        ..Default::default()
    }
}

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
