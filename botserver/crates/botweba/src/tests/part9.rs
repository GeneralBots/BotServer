use super::*;

impl QueuePage {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn url_pattern() -> &'static str {
        "/queue"
    }

    pub fn queue_panel() -> Locator {
        Locator::css(".queue-panel")
    }

    pub fn queue_count() -> Locator {
        Locator::css(".queue-count")
    }

    pub fn take_next_button() -> Locator {
        Locator::css("button.take-next")
    }
}
