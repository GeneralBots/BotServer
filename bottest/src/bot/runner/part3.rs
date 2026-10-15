//! Split from `runner.rs` — 30 lines.

use super::*;

impl Default for BotRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub customer_id: Uuid,
    pub channel: Channel,
    pub message_count: usize,
    pub state: ConversationState,
    pub duration: Duration,
}

impl Clone for SessionState {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            customer: self.customer.clone(),
            channel: self.channel,
            context: self.context.clone(),
            conversation_state: self.conversation_state,
            message_count: self.message_count,
            started_at: self.started_at,
        }
    }
}
