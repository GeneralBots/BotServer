use super::*;

impl InstagramAdapter {
    pub(super) fn new() -> Self { Self }
    pub(crate) async fn send_message(&self, _response: botcore::shared::BotResponse) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Instagram adapter not available - register at botserver layer".into())
    }
}
