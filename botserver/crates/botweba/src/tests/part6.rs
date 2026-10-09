use super::*;

impl LoginPage {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn url_pattern() -> &'static str {
        "/login"
    }

    pub fn email_input() -> Locator {
        Locator::id("email")
    }

    pub fn password_input() -> Locator {
        Locator::id("password")
    }

    pub fn login_button() -> Locator {
        Locator::css("button[type='submit']")
    }

    pub fn error_message() -> Locator {
        Locator::css(".error-message")
    }
}
