//! Split from `teams.rs` (#1504).

use super::*;

impl MockTeams {
    #[must_use]
    pub fn simulate_reaction(
        &self,
        from_id: &str,
        from_name: &str,
        message_id: &str,
        reaction: &str,
    ) -> Activity {
        let conversation_id = format!("conv-{}", Uuid::new_v4());

        Activity {
            kind: "messageReaction".to_string(),
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            local_timestamp: None,
            service_url: self.service_url.clone(),
            channel_id: "msteams".to_string(),
            from: ChannelAccount {
                id: from_id.to_string(),
                name: Some(from_name.to_string()),
                aad_object_id: None,
                role: None,
            },
            conversation: ConversationAccount {
                id: conversation_id,
                name: None,
                conversation_type: Some("personal".to_string()),
                is_group: Some(false),
                tenant_id: Some(self.tenant_id.clone()),
            },
            recipient: ChannelAccount {
                id: self.bot_id.clone(),
                name: Some(self.bot_name.clone()),
                aad_object_id: None,
                role: Some("bot".to_string()),
            },
            text: None,
            text_format: None,
            locale: None,
            attachments: None,
            entities: None,
            channel_data: None,
            action: None,
            reply_to_id: Some(message_id.to_string()),
            value: Some(serde_json::json!({
                "reactionsAdded": [{
                    "type": reaction
                }]
            })),
            name: None,
        }
    }

    pub async fn expect_error(&self, code: &str, message: &str) {
        let error_response = ErrorResponse {
            error: ErrorBody {
                code: code.to_string(),
                message: message.to_string(),
            },
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/v3/conversations/.+/activities"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_unauthorized(&self) {
        self.expect_error("Unauthorized", "Token validation failed")
            .await;
    }

    pub async fn expect_not_found(&self) {
        self.expect_error("NotFound", "Conversation not found")
            .await;
    }

    #[must_use]
    pub fn sent_activities(&self) -> Vec<Activity> {
        self.sent_activities.lock().unwrap().clone()
    }

    #[must_use]
    pub fn sent_activities_containing(&self, text: &str) -> Vec<Activity> {
        self.sent_activities
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.text.as_ref().is_some_and(|t| t.contains(text)))
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn last_sent_activity(&self) -> Option<Activity> {
        self.sent_activities.lock().unwrap().last().cloned()
    }

    pub fn clear_sent_activities(&self) {
        self.sent_activities.lock().unwrap().clear();
    }

    pub fn register_conversation(&self, info: ConversationInfo) {
        self.conversations
            .lock()
            .unwrap()
            .insert(info.id.clone(), info);
    }

    #[must_use]
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub fn service_url(&self) -> String {
        self.service_url.clone()
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn bot_id(&self) -> &str {
        &self.bot_id
    }

    #[must_use]
    pub fn bot_name(&self) -> &str {
        &self.bot_name
    }

    #[must_use]
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub fn verify(&self) -> Result<()> {
        let store = self.expectations.lock().unwrap();
        for exp in store.values() {
            exp.verify()?;
        }
        Ok(())
    }

    pub async fn reset(&self) {
        self.server.reset().await;
        self.sent_activities.lock().unwrap().clear();
        self.conversations.lock().unwrap().clear();
        self.expectations.lock().unwrap().clear();
        self.setup_default_routes().await;
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }
}
