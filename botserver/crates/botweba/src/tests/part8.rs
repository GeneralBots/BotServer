use super::*;

impl ChatPage {
    pub fn new(base_url: &str, bot_name: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            bot_name: bot_name.to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn bot_name(&self) -> &str {
        &self.bot_name
    }

    pub fn url_pattern() -> &'static str {
        "/chat/"
    }

    pub fn chat_input() -> Locator {
        Locator::id("chat-input")
    }

    pub fn send_button() -> Locator {
        Locator::css("button.send-message")
    }

    pub fn bot_message() -> Locator {
        Locator::css(".message.bot")
    }

    pub fn typing_indicator() -> Locator {
        Locator::css(".typing-indicator")
    }
}
