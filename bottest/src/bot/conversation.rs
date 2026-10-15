use super::{
    AssertionRecord, AssertionResult, BotResponse, ConversationConfig, ConversationRecord,
    ConversationState, RecordedMessage, ResponseContentType,
};
use crate::fixtures::{Channel, Customer, MessageDirection};
use crate::harness::TestContext;
use crate::mocks::MockLLM;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct ConversationBuilder {
    bot_name: String,
    customer: Option<Customer>,
    channel: Channel,
    config: ConversationConfig,
    initial_context: HashMap<String, serde_json::Value>,
    mock_llm: Option<Arc<MockLLM>>,
}

impl ConversationBuilder {
    pub fn new(bot_name: &str) -> Self {
        Self {
            bot_name: bot_name.to_string(),
            customer: None,
            channel: Channel::WhatsApp,
            config: ConversationConfig::default(),
            initial_context: HashMap::new(),
            mock_llm: None,
        }
    }

    pub fn with_customer(mut self, customer: Customer) -> Self {
        self.customer = Some(customer);
        self
    }

    pub const fn on_channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }

    pub fn with_config(mut self, config: ConversationConfig) -> Self {
        self.config = config;
        self
    }

    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.response_timeout = timeout;
        self
    }

    pub fn with_context(mut self, key: &str, value: serde_json::Value) -> Self {
        self.initial_context.insert(key.to_string(), value);
        self
    }

    pub const fn without_recording(mut self) -> Self {
        self.config.record = false;
        self
    }

    pub const fn with_real_llm(mut self) -> Self {
        self.config.use_mock_llm = false;
        self
    }

    pub fn with_mock_llm(mut self, mock: Arc<MockLLM>) -> Self {
        self.mock_llm = Some(mock);
        self.config.use_mock_llm = true;
        self
    }

    pub fn build(self) -> ConversationTest {
        let customer = self.customer.unwrap_or_else(|| Customer {
            channel: self.channel,
            ..Default::default()
        });

        let bot_name_for_record = self.bot_name.clone();

        ConversationTest {
            id: Uuid::new_v4(),
            bot_name: self.bot_name,
            customer,
            channel: self.channel,
            config: self.config,
            state: ConversationState::Initial,
            responses: Vec::new(),
            sent_messages: Vec::new(),
            record: ConversationRecord {
                id: Uuid::new_v4(),
                bot_name: bot_name_for_record,
                started_at: Utc::now(),
                ended_at: None,
                messages: Vec::new(),
                assertions: Vec::new(),
                passed: true,
            },
            context: self.initial_context,
            last_response: None,
            last_latency: None,
            mock_llm: self.mock_llm,
            llm_url: None,
        }
    }
}

pub struct ConversationTest {
    id: Uuid,
    bot_name: String,
    customer: Customer,
    channel: Channel,
    config: ConversationConfig,
    state: ConversationState,
    responses: Vec<BotResponse>,
    sent_messages: Vec<String>,
    record: ConversationRecord,
    context: HashMap<String, serde_json::Value>,
    last_response: Option<BotResponse>,
    last_latency: Option<Duration>,
    mock_llm: Option<Arc<MockLLM>>,
    llm_url: Option<String>,
}

mod part2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_builder() {
        let conv = ConversationBuilder::new("test-bot")
            .on_channel(Channel::Web)
            .with_timeout(Duration::from_secs(10))
            .build();

        assert_eq!(conv.bot_name(), "test-bot");
        assert_eq!(conv.channel(), Channel::Web);
        assert_eq!(conv.state(), ConversationState::Initial);
    }

    #[test]
    fn test_conversation_test_new() {
        let conv = ConversationTest::new("my-bot");
        assert_eq!(conv.bot_name(), "my-bot");
        assert!(conv.responses().is_empty());
        assert!(conv.sent_messages().is_empty());
    }

    #[tokio::test]
    async fn test_user_says() {
        let mut conv = ConversationTest::new("test-bot");
        conv.user_says("Hello").await;

        assert_eq!(conv.sent_messages().len(), 1);
        assert_eq!(conv.sent_messages()[0], "Hello");
        assert_eq!(conv.responses().len(), 1);
        assert!(conv.last_response().is_some());
    }

    #[tokio::test]
    async fn test_assert_response_contains() {
        let mut conv = ConversationTest::new("test-bot");
        conv.user_says("test").await;
        conv.assert_response_contains("Response");

        assert!(conv.all_passed());
    }

    #[tokio::test]
    async fn test_assert_response_not_contains() {
        let mut conv = ConversationTest::new("test-bot");
        conv.user_says("test").await;
        conv.assert_response_not_contains("nonexistent");

        assert!(conv.all_passed());
    }

    #[tokio::test]
    async fn test_conversation_recording() {
        let mut conv = ConversationBuilder::new("test-bot").build();
        conv.user_says("Hello").await;
        conv.user_says("How are you?").await;

        let record = conv.record();
        assert_eq!(record.messages.len(), 4);
    }

    #[tokio::test]
    async fn test_conversation_without_recording() {
        let mut conv = ConversationBuilder::new("test-bot")
            .without_recording()
            .build();
        conv.user_says("Hello").await;

        let record = conv.record();
        assert!(record.messages.is_empty());
    }

    #[test]
    fn test_context_variables() {
        let mut conv = ConversationTest::new("test-bot");
        conv.set_context("user_name", serde_json::json!("Alice"));

        let value = conv.get_context("user_name");
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str().unwrap(), "Alice");
    }

    #[tokio::test]
    async fn test_end_conversation() {
        let mut conv = ConversationTest::new("test-bot");
        conv.user_says("bye").await;
        conv.end();

        assert_eq!(conv.state(), ConversationState::Ended);
        assert!(conv.record().ended_at.is_some());
    }

    #[tokio::test]
    async fn test_failed_assertions() {
        let mut conv = ConversationTest::new("test-bot");
        conv.user_says("test").await;
        conv.assert_response_equals("this will not match");

        assert!(!conv.all_passed());
        assert_eq!(conv.failed_assertions().len(), 1);
    }

    #[tokio::test]
    async fn test_response_metadata() {
        let conv = ConversationBuilder::new("test-bot")
            .on_channel(Channel::WhatsApp)
            .build();

        let metadata = conv.build_response_metadata();
        assert_eq!(
            metadata.get("bot_name").unwrap().as_str().unwrap(),
            "test-bot"
        );
        assert!(metadata
            .get("channel")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("WhatsApp"));
    }

    #[tokio::test]
    async fn test_multiple_messages_flow() {
        let mut conv = ConversationTest::new("support-bot");

        conv.user_says("Hi").await;
        conv.assert_response_contains("Response");

        conv.user_says("I need help").await;
        conv.assert_response_contains("Response");

        conv.user_says("Thanks, bye").await;
        conv.end();

        assert_eq!(conv.sent_messages().len(), 3);
        assert_eq!(conv.responses().len(), 3);
        assert!(conv.all_passed());
    }

    #[tokio::test]
    async fn test_response_time_assertion() {
        let mut conv = ConversationTest::new("test-bot");
        conv.user_says("quick test").await;
        conv.assert_response_within(Duration::from_secs(5));

        assert!(conv.all_passed());
    }

    #[tokio::test]
    async fn test_response_count_assertion() {
        let mut conv = ConversationTest::new("test-bot");
        conv.user_says("one").await;
        conv.user_says("two").await;
        conv.assert_response_count(2);

        assert!(conv.all_passed());
    }

    #[tokio::test]
    async fn test_customer_info_in_metadata() {
        let customer = Customer {
            id: Uuid::new_v4(),
            phone: Some("+15551234567".to_string()),
            ..Default::default()
        };

        let conv = ConversationBuilder::new("test-bot")
            .with_customer(customer.clone())
            .build();

        assert_eq!(conv.customer().id, customer.id);
        assert_eq!(conv.customer().phone, customer.phone);
    }
}
