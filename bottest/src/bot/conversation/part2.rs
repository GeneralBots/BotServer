//! Split from `conversation.rs` — 415 lines.

use super::*;

impl ConversationTest {
    pub fn new(bot_name: &str) -> Self {
        ConversationBuilder::new(bot_name).build()
    }

    pub fn with_context(ctx: &TestContext, bot_name: &str) -> Result<Self> {
        let mut conv = ConversationBuilder::new(bot_name).build();
        conv.llm_url = Some(ctx.llm_url());
        Ok(conv)
    }

    pub const fn id(&self) -> Uuid {
        self.id
    }

    pub fn bot_name(&self) -> &str {
        &self.bot_name
    }

    pub const fn customer(&self) -> &Customer {
        &self.customer
    }

    pub const fn channel(&self) -> Channel {
        self.channel
    }

    pub const fn state(&self) -> ConversationState {
        self.state
    }

    pub fn responses(&self) -> &[BotResponse] {
        &self.responses
    }

    pub fn sent_messages(&self) -> &[String] {
        &self.sent_messages
    }

    pub const fn last_response(&self) -> Option<&BotResponse> {
        self.last_response.as_ref()
    }

    pub const fn last_latency(&self) -> Option<Duration> {
        self.last_latency
    }

    pub const fn record(&self) -> &ConversationRecord {
        &self.record
    }

    pub async fn user_says(&mut self, message: &str) -> &mut Self {
        self.sent_messages.push(message.to_string());

        if self.config.record {
            self.record.messages.push(RecordedMessage {
                timestamp: Utc::now(),
                direction: MessageDirection::Incoming,
                content: message.to_string(),
                latency_ms: None,
            });
        }

        self.state = ConversationState::WaitingForBot;

        let start = Instant::now();
        let response = self.get_bot_response(message).await;
        let latency = start.elapsed();

        self.last_latency = Some(latency);
        self.last_response = Some(response.clone());
        self.responses.push(response.clone());

        if self.config.record {
            self.record.messages.push(RecordedMessage {
                timestamp: Utc::now(),
                direction: MessageDirection::Outgoing,
                content: response.content,
                latency_ms: Some(latency.as_millis() as u64),
            });
        }

        self.state = ConversationState::WaitingForUser;
        self
    }

    pub(crate) async fn get_bot_response(&self, user_message: &str) -> BotResponse {
        let start = Instant::now();

        if self.config.use_mock_llm {
            if let Some(ref mock) = self.mock_llm {
                let mock_url = mock.url();
                if let Ok(content) = self.call_llm_api(&mock_url, user_message).await {
                    return BotResponse {
                        id: Uuid::new_v4(),
                        content,
                        content_type: ResponseContentType::Text,
                        metadata: self.build_response_metadata(),
                        latency_ms: start.elapsed().as_millis() as u64,
                    };
                }
            } else if let Some(ref llm_url) = self.llm_url {
                if let Ok(content) = self.call_llm_api(llm_url, user_message).await {
                    return BotResponse {
                        id: Uuid::new_v4(),
                        content,
                        content_type: ResponseContentType::Text,
                        metadata: self.build_response_metadata(),
                        latency_ms: start.elapsed().as_millis() as u64,
                    };
                }
            }
        }

        BotResponse {
            id: Uuid::new_v4(),
            content: format!("Response to: {user_message}"),
            content_type: ResponseContentType::Text,
            metadata: self.build_response_metadata(),
            latency_ms: start.elapsed().as_millis() as u64,
        }
    }

    pub(crate) async fn call_llm_api(&self, llm_url: &str, message: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(self.config.response_timeout)
            .build()?;

        let request_body = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {
                    "role": "system",
                    "content": format!("You are a helpful assistant for bot '{}'.", self.bot_name)
                },
                {
                    "role": "user",
                    "content": message
                }
            ]
        });

        let response = client
            .post(format!("{llm_url}/v1/chat/completions"))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No response")
            .to_string();

        Ok(content)
    }

    pub(crate) fn build_response_metadata(&self) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "bot_name".to_string(),
            serde_json::Value::String(self.bot_name.clone()),
        );
        metadata.insert(
            "customer_id".to_string(),
            serde_json::Value::String(self.customer.id.to_string()),
        );
        metadata.insert(
            "channel".to_string(),
            serde_json::Value::String(format!("{:?}", self.channel)),
        );
        metadata.insert(
            "conversation_id".to_string(),
            serde_json::Value::String(self.id.to_string()),
        );
        metadata
    }

    pub fn assert_response_contains(&mut self, text: &str) -> &mut Self {
        let result = if let Some(ref response) = self.last_response {
            if response.content.contains(text) {
                AssertionResult::pass(&format!("Response contains '{text}'"))
            } else {
                AssertionResult::fail(
                    &format!("Response should contain '{text}'"),
                    text,
                    &response.content,
                )
            }
        } else {
            AssertionResult::fail("No response to check", text, "<no response>")
        };

        self.record_assertion("contains", &result);
        self
    }

    pub fn assert_response_equals(&mut self, text: &str) -> &mut Self {
        let result = if let Some(ref response) = self.last_response {
            if response.content == text {
                AssertionResult::pass(&format!("Response equals '{text}'"))
            } else {
                AssertionResult::fail(
                    "Response should equal expected text",
                    text,
                    &response.content,
                )
            }
        } else {
            AssertionResult::fail("No response to check", text, "<no response>")
        };

        self.record_assertion("equals", &result);
        self
    }

    pub fn assert_response_matches(&mut self, pattern: &str) -> &mut Self {
        let result = if let Some(ref response) = self.last_response {
            match regex::Regex::new(pattern) {
                Ok(re) => {
                    if re.is_match(&response.content) {
                        AssertionResult::pass(&format!("Response matches pattern '{pattern}'"))
                    } else {
                        AssertionResult::fail(
                            &format!("Response should match pattern '{pattern}'"),
                            pattern,
                            &response.content,
                        )
                    }
                }
                Err(e) => AssertionResult::fail(
                    &format!("Invalid regex pattern: {e}"),
                    pattern,
                    "<invalid pattern>",
                ),
            }
        } else {
            AssertionResult::fail("No response to check", pattern, "<no response>")
        };

        self.record_assertion("matches", &result);
        self
    }

    pub fn assert_response_not_contains(&mut self, text: &str) -> &mut Self {
        let result = if let Some(ref response) = self.last_response {
            if response.content.contains(text) {
                AssertionResult::fail(
                    &format!("Response should not contain '{text}'"),
                    &format!("not containing '{text}'"),
                    &response.content,
                )
            } else {
                AssertionResult::pass(&format!("Response does not contain '{text}'"))
            }
        } else {
            AssertionResult::pass("No response (nothing to contain)")
        };

        self.record_assertion("not_contains", &result);
        self
    }

    pub fn assert_transferred_to_human(&mut self) -> &mut Self {
        let is_transferred = self.state == ConversationState::Transferred
            || self.last_response.as_ref().is_some_and(|r| {
                r.content.to_lowercase().contains("transfer")
                    || r.content.to_lowercase().contains("human")
                    || r.content.to_lowercase().contains("agent")
            });

        let result = if is_transferred {
            self.state = ConversationState::Transferred;
            AssertionResult::pass("Conversation transferred to human")
        } else {
            AssertionResult::fail(
                "Should be transferred to human",
                "transferred",
                "not transferred",
            )
        };

        self.record_assertion("transferred", &result);
        self
    }

    pub fn assert_queue_position(&mut self, expected: usize) -> &mut Self {
        let actual = self
            .context
            .get("queue_position")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize;

        let result = if actual == expected {
            AssertionResult::pass(&format!("Queue position is {expected}"))
        } else {
            AssertionResult::fail(
                "Queue position mismatch",
                &expected.to_string(),
                &actual.to_string(),
            )
        };

        self.record_assertion("queue_position", &result);
        self
    }

    pub fn assert_response_within(&mut self, max_duration: Duration) -> &mut Self {
        let result = if let Some(latency) = self.last_latency {
            if latency <= max_duration {
                AssertionResult::pass(&format!("Response within {max_duration:?}"))
            } else {
                AssertionResult::fail(
                    "Response too slow",
                    &format!("{max_duration:?}"),
                    &format!("{latency:?}"),
                )
            }
        } else {
            AssertionResult::fail(
                "No latency recorded",
                &format!("{max_duration:?}"),
                "<no latency>",
            )
        };

        self.record_assertion("response_time", &result);
        self
    }

    pub fn assert_response_count(&mut self, expected: usize) -> &mut Self {
        let actual = self.responses.len();

        let result = if actual == expected {
            AssertionResult::pass(&format!("Response count is {expected}"))
        } else {
            AssertionResult::fail(
                "Response count mismatch",
                &expected.to_string(),
                &actual.to_string(),
            )
        };

        self.record_assertion("response_count", &result);
        self
    }

    pub fn assert_response_type(&mut self, expected: ResponseContentType) -> &mut Self {
        let result = if let Some(ref response) = self.last_response {
            if response.content_type == expected {
                AssertionResult::pass(&format!("Response type is {expected:?}"))
            } else {
                AssertionResult::fail(
                    "Response type mismatch",
                    &format!("{expected:?}"),
                    &format!("{:?}", response.content_type),
                )
            }
        } else {
            AssertionResult::fail(
                "No response to check",
                &format!("{expected:?}"),
                "<no response>",
            )
        };

        self.record_assertion("response_type", &result);
        self
    }

    pub fn set_context(&mut self, key: &str, value: serde_json::Value) -> &mut Self {
        self.context.insert(key.to_string(), value);
        self
    }

    pub fn get_context(&self, key: &str) -> Option<&serde_json::Value> {
        self.context.get(key)
    }

    pub fn end(&mut self) -> &mut Self {
        self.state = ConversationState::Ended;
        self.record.ended_at = Some(Utc::now());
        self
    }

    pub const fn all_passed(&self) -> bool {
        self.record.passed
    }

    pub fn failed_assertions(&self) -> Vec<&AssertionRecord> {
        self.record
            .assertions
            .iter()
            .filter(|a| !a.passed)
            .collect()
    }

    pub(crate) fn record_assertion(&mut self, assertion_type: &str, result: &AssertionResult) {
        if !result.passed {
            self.record.passed = false;
        }

        if self.config.record {
            self.record.assertions.push(AssertionRecord {
                timestamp: Utc::now(),
                assertion_type: assertion_type.to_string(),
                passed: result.passed,
                message: result.message.clone(),
            });
        }
    }
}
