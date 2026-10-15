
use anyhow::Result;
use std::time::Duration;

use super::browser::{Browser, Element};
use super::Locator;

#[async_trait::async_trait]
pub trait Page {
    fn url_pattern(&self) -> &str;

    async fn is_current(&self, browser: &Browser) -> Result<bool> {
        let url = browser.current_url().await?;
        Ok(url.contains(self.url_pattern()))
    }

    async fn wait_for_load(&self, browser: &Browser) -> Result<()>;
}


pub struct LoginPage {
    pub base_url: String,
}

impl LoginPage {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn navigate(&self, browser: &Browser) -> Result<()> {
        browser.goto(&format!("{}/login", self.base_url)).await
    }

    #[must_use]
    pub fn email_input() -> Locator {
        Locator::css("#email, input[name='email'], input[type='email']")
    }

    #[must_use]
    pub fn password_input() -> Locator {
        Locator::css("#password, input[name='password'], input[type='password']")
    }

    #[must_use]
    pub fn login_button() -> Locator {
        Locator::css(
            "#login-button, button[type='submit'], input[type='submit'], .login-btn, .btn-login",
        )
    }

    #[must_use]
    pub fn error_message() -> Locator {
        Locator::css(".error, .error-message, .alert-error, .alert-danger, [role='alert']")
    }

    pub async fn enter_email(&self, browser: &Browser, email: &str) -> Result<()> {
        browser.fill(Self::email_input(), email).await
    }

    pub async fn enter_password(&self, browser: &Browser, password: &str) -> Result<()> {
        browser.fill(Self::password_input(), password).await
    }

    pub async fn click_login(&self, browser: &Browser) -> Result<()> {
        browser.click(Self::login_button()).await
    }

    pub async fn login(&self, browser: &Browser, email: &str, password: &str) -> Result<()> {
        self.navigate(browser).await?;
        self.wait_for_load(browser).await?;
        self.enter_email(browser, email).await?;
        self.enter_password(browser, password).await?;
        self.click_login(browser).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    pub async fn has_error(&self, browser: &Browser) -> bool {
        browser.exists(Self::error_message()).await
    }

    pub async fn get_error_message(&self, browser: &Browser) -> Result<String> {
        browser.text(Self::error_message()).await
    }
}

#[async_trait::async_trait]
impl Page for LoginPage {
    fn url_pattern(&self) -> &'static str {
        "/login"
    }

    async fn wait_for_load(&self, browser: &Browser) -> Result<()> {
        browser.wait_for(Self::email_input()).await?;
        browser.wait_for(Self::password_input()).await?;
        Ok(())
    }
}


pub struct DashboardPage {
    pub base_url: String,
}

impl DashboardPage {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn navigate(&self, browser: &Browser) -> Result<()> {
        browser.goto(&format!("{}/dashboard", self.base_url)).await
    }

    #[must_use]
    pub fn stats_cards() -> Locator {
        Locator::css(".stats-card, .dashboard-stat, .metric-card")
    }

    #[must_use]
    pub fn nav_menu() -> Locator {
        Locator::css("nav, .nav, .sidebar, .navigation")
    }

    #[must_use]
    pub fn user_profile() -> Locator {
        Locator::css(".user-profile, .user-menu, .profile-dropdown, .avatar")
    }

    #[must_use]
    pub fn logout_button() -> Locator {
        Locator::css(".logout, .logout-btn, #logout, a[href*='logout'], button:contains('Logout')")
    }

    pub async fn get_nav_items(&self, browser: &Browser) -> Result<Vec<Element>> {
        browser
            .find_all(Locator::css("nav a, .nav-item, .menu-item"))
            .await
    }

    pub async fn navigate_to(&self, browser: &Browser, menu_text: &str) -> Result<()> {
        let locator = Locator::xpath(&format!("//nav//a[contains(text(), '{menu_text}')]"));
        browser.click(locator).await
    }

    pub async fn logout(&self, browser: &Browser) -> Result<()> {
        if browser.exists(Self::user_profile()).await {
            let _ = browser.click(Self::user_profile()).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        browser.click(Self::logout_button()).await
    }
}

#[async_trait::async_trait]
impl Page for DashboardPage {
    fn url_pattern(&self) -> &'static str {
        "/dashboard"
    }

    async fn wait_for_load(&self, browser: &Browser) -> Result<()> {
        browser.wait_for(Self::nav_menu()).await?;
        Ok(())
    }
}


pub struct ChatPage {
    pub base_url: String,
    pub bot_name: String,
}

impl ChatPage {
    #[must_use]
    pub fn new(base_url: &str, bot_name: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            bot_name: bot_name.to_string(),
        }
    }

    pub async fn navigate(&self, browser: &Browser) -> Result<()> {
        browser
            .goto(&format!("{}/chat/{}", self.base_url, self.bot_name))
            .await
    }

    #[must_use]
    pub fn chat_input() -> Locator {
        Locator::css(
            "#chat-input, .chat-input, input[name='message'], textarea[name='message'], .message-input",
        )
    }

    #[must_use]
    pub fn send_button() -> Locator {
        Locator::css("#send, .send-btn, button[type='submit'], .send-message")
    }

    #[must_use]
    pub fn message_list() -> Locator {
        Locator::css(".messages, .message-list, .chat-messages, #messages")
    }

    #[must_use]
    pub fn bot_message() -> Locator {
        Locator::css(".bot-message, .message-bot, .assistant-message, [data-role='bot']")
    }

    #[must_use]
    pub fn user_message() -> Locator {
        Locator::css(".user-message, .message-user, [data-role='user']")
    }

    #[must_use]
    pub fn typing_indicator() -> Locator {
        Locator::css(".typing, .typing-indicator, .is-typing, [data-typing]")
    }

    #[must_use]
    pub fn file_upload_button() -> Locator {
        Locator::css(".upload-btn, .file-upload, input[type='file'], .attach-file")
    }

    #[must_use]
    pub fn quick_reply_buttons() -> Locator {
        Locator::css(".quick-replies, .quick-reply, .suggested-reply")
    }

    pub async fn send_message(&self, browser: &Browser, message: &str) -> Result<()> {
        browser.fill(Self::chat_input(), message).await?;
        browser.click(Self::send_button()).await?;
        Ok(())
    }

    pub async fn wait_for_response(&self, browser: &Browser, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if browser.exists(Self::typing_indicator()).await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        while start.elapsed() < timeout {
            if !browser.exists(Self::typing_indicator()).await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        anyhow::bail!("Timeout waiting for bot response")
    }

    pub async fn get_bot_messages(&self, browser: &Browser) -> Result<Vec<String>> {
        let elements = browser.find_all(Self::bot_message()).await?;
        let mut messages = Vec::new();
        for elem in elements {
            if let Ok(text) = elem.text().await {
                messages.push(text);
            }
        }
        Ok(messages)
    }

    pub async fn get_user_messages(&self, browser: &Browser) -> Result<Vec<String>> {
        let elements = browser.find_all(Self::user_message()).await?;
        let mut messages = Vec::new();
        for elem in elements {
            if let Ok(text) = elem.text().await {
                messages.push(text);
            }
        }
        Ok(messages)
    }

    pub async fn get_last_bot_message(&self, browser: &Browser) -> Result<String> {
        let messages = self.get_bot_messages(browser).await?;
        messages
            .last()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No bot messages found"))
    }

    pub async fn is_typing(&self, browser: &Browser) -> bool {
        browser.exists(Self::typing_indicator()).await
    }

    pub async fn click_quick_reply(&self, browser: &Browser, text: &str) -> Result<()> {
        let locator = Locator::xpath(&format!(
            "//*[contains(@class, 'quick-reply') and contains(text(), '{text}')]"
        ));
        browser.click(locator).await
    }
}

#[async_trait::async_trait]
impl Page for ChatPage {
    fn url_pattern(&self) -> &'static str {
        "/chat/"
    }

    async fn wait_for_load(&self, browser: &Browser) -> Result<()> {
        browser.wait_for(Self::chat_input()).await?;
        browser.wait_for(Self::message_list()).await?;
        Ok(())
    }
}

mod part2;
pub use part2::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_page_locators() {
        let _ = LoginPage::email_input();
        let _ = LoginPage::password_input();
        let _ = LoginPage::login_button();
        let _ = LoginPage::error_message();
    }

    #[test]
    fn test_chat_page_locators() {
        let _ = ChatPage::chat_input();
        let _ = ChatPage::send_button();
        let _ = ChatPage::bot_message();
        let _ = ChatPage::typing_indicator();
    }

    #[test]
    fn test_queue_page_locators() {
        let _ = QueuePage::queue_panel();
        let _ = QueuePage::queue_count();
        let _ = QueuePage::take_next_button();
    }

    #[test]
    fn test_page_url_patterns() {
        let login = LoginPage::new("http://localhost:4242");
        assert_eq!(login.url_pattern(), "/login");

        let dashboard = DashboardPage::new("http://localhost:4242");
        assert_eq!(dashboard.url_pattern(), "/dashboard");

        let chat = ChatPage::new("http://localhost:4242", "test-bot");
        assert_eq!(chat.url_pattern(), "/chat/");

        let queue = QueuePage::new("http://localhost:4242");
        assert_eq!(queue.url_pattern(), "/queue");

        let bots = BotManagementPage::new("http://localhost:4242");
        assert_eq!(bots.url_pattern(), "/admin/bots");
    }
}
