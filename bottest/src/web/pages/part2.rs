//! Split from `mod.rs` — 309 lines.

use super::*;

pub struct QueuePage {
    pub base_url: String,
}

impl QueuePage {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn navigate(&self, browser: &Browser) -> Result<()> {
        browser.goto(&format!("{}/queue", self.base_url)).await
    }

    #[must_use]
    pub fn queue_panel() -> Locator {
        Locator::css(".queue-panel, .queue-container, #queue-panel")
    }

    #[must_use]
    pub fn queue_count() -> Locator {
        Locator::css(".queue-count, .waiting-count, #queue-count")
    }

    #[must_use]
    pub fn queue_entry() -> Locator {
        Locator::css(".queue-entry, .queue-item, .waiting-customer")
    }

    #[must_use]
    pub fn take_next_button() -> Locator {
        Locator::css(".take-next, #take-next, button:contains('Take Next')")
    }

    pub async fn get_queue_count(&self, browser: &Browser) -> Result<u32> {
        let text = browser.text(Self::queue_count()).await?;
        text.parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Failed to parse queue count: {text}"))
    }

    pub async fn get_queue_entries(&self, browser: &Browser) -> Result<Vec<Element>> {
        browser.find_all(Self::queue_entry()).await
    }

    pub async fn take_next(&self, browser: &Browser) -> Result<()> {
        browser.click(Self::take_next_button()).await
    }
}

#[async_trait::async_trait]
impl Page for QueuePage {
    fn url_pattern(&self) -> &'static str {
        "/queue"
    }

    async fn wait_for_load(&self, browser: &Browser) -> Result<()> {
        browser.wait_for(Self::queue_panel()).await?;
        Ok(())
    }
}

pub struct BotManagementPage {
    pub base_url: String,
}

impl BotManagementPage {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn navigate(&self, browser: &Browser) -> Result<()> {
        browser.goto(&format!("{}/admin/bots", self.base_url)).await
    }

    #[must_use]
    pub fn bot_list() -> Locator {
        Locator::css(".bot-list, .bots-container, #bots")
    }

    #[must_use]
    pub fn bot_item() -> Locator {
        Locator::css(".bot-item, .bot-card, .bot-entry")
    }

    #[must_use]
    pub fn create_bot_button() -> Locator {
        Locator::css(".create-bot, .new-bot, #create-bot, button:contains('Create')")
    }

    #[must_use]
    pub fn bot_name_input() -> Locator {
        Locator::css("#bot-name, input[name='name'], .bot-name-input")
    }

    #[must_use]
    pub fn bot_description_input() -> Locator {
        Locator::css("#bot-description, textarea[name='description'], .bot-description-input")
    }

    #[must_use]
    pub fn save_button() -> Locator {
        Locator::css(".save-btn, button[type='submit'], #save, button:contains('Save')")
    }

    pub async fn get_bots(&self, browser: &Browser) -> Result<Vec<Element>> {
        browser.find_all(Self::bot_item()).await
    }

    pub async fn click_create_bot(&self, browser: &Browser) -> Result<()> {
        browser.click(Self::create_bot_button()).await
    }

    pub async fn create_bot(&self, browser: &Browser, name: &str, description: &str) -> Result<()> {
        self.click_create_bot(browser).await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        browser.fill(Self::bot_name_input(), name).await?;
        browser
            .fill(Self::bot_description_input(), description)
            .await?;
        browser.click(Self::save_button()).await?;
        Ok(())
    }

    pub async fn edit_bot(&self, browser: &Browser, bot_name: &str) -> Result<()> {
        let locator = Locator::xpath(&format!(
            "//*[contains(@class, 'bot-item') and contains(., '{bot_name}')]//button[contains(@class, 'edit')]"
        ));
        browser.click(locator).await
    }
}

#[async_trait::async_trait]
impl Page for BotManagementPage {
    fn url_pattern(&self) -> &'static str {
        "/admin/bots"
    }

    async fn wait_for_load(&self, browser: &Browser) -> Result<()> {
        browser.wait_for(Self::bot_list()).await?;
        Ok(())
    }
}

pub struct KnowledgeBasePage {
    pub base_url: String,
}

impl KnowledgeBasePage {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn navigate(&self, browser: &Browser) -> Result<()> {
        browser.goto(&format!("{}/admin/kb", self.base_url)).await
    }

    #[must_use]
    pub fn kb_list() -> Locator {
        Locator::css(".kb-list, .knowledge-base-list, #kb-list")
    }

    #[must_use]
    pub fn kb_entry() -> Locator {
        Locator::css(".kb-entry, .kb-item, .knowledge-entry")
    }

    #[must_use]
    pub fn upload_button() -> Locator {
        Locator::css(".upload-btn, #upload, button:contains('Upload')")
    }

    #[must_use]
    pub fn file_input() -> Locator {
        Locator::css("input[type='file']")
    }

    #[must_use]
    pub fn search_input() -> Locator {
        Locator::css(".search-input, #search, input[placeholder*='search']")
    }

    pub async fn get_entries(&self, browser: &Browser) -> Result<Vec<Element>> {
        browser.find_all(Self::kb_entry()).await
    }

    pub async fn search(&self, browser: &Browser, query: &str) -> Result<()> {
        browser.fill(Self::search_input(), query).await
    }
}

#[async_trait::async_trait]
impl Page for KnowledgeBasePage {
    fn url_pattern(&self) -> &'static str {
        "/admin/kb"
    }

    async fn wait_for_load(&self, browser: &Browser) -> Result<()> {
        browser.wait_for(Self::kb_list()).await?;
        Ok(())
    }
}

pub struct AnalyticsPage {
    pub base_url: String,
}

impl AnalyticsPage {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub async fn navigate(&self, browser: &Browser) -> Result<()> {
        browser
            .goto(&format!("{}/admin/analytics", self.base_url))
            .await
    }

    #[must_use]
    pub fn charts_container() -> Locator {
        Locator::css(".charts, .analytics-charts, #charts")
    }

    #[must_use]
    pub fn date_range_picker() -> Locator {
        Locator::css(".date-range, .date-picker, #date-range")
    }

    #[must_use]
    pub fn metric_card() -> Locator {
        Locator::css(".metric-card, .analytics-metric, .stat-card")
    }

    pub async fn get_metrics(&self, browser: &Browser) -> Result<Vec<Element>> {
        browser.find_all(Self::metric_card()).await
    }
}

#[async_trait::async_trait]
impl Page for AnalyticsPage {
    fn url_pattern(&self) -> &'static str {
        "/admin/analytics"
    }

    async fn wait_for_load(&self, browser: &Browser) -> Result<()> {
        browser.wait_for(Self::charts_container()).await?;
        Ok(())
    }
}

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
