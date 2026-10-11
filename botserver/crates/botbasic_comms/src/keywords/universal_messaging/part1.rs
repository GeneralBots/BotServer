use super::*;

impl WhatsAppAdapter {
    pub(super) fn new(_state: &AppState, _bot_id: uuid::Uuid) -> Self { Self }
    pub(crate) async fn send_message(&self, _response: botcore::shared::BotResponse) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("WhatsApp adapter not available - register at botserver layer".into())
    }
}
