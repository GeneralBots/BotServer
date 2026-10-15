//! Split from `whatsapp.rs` (#1504).

use super::*;

impl MockWhatsApp {
    pub async fn expect_error(&self, code: u32, message: &str) {
        let error_response = ErrorResponse {
            error: ErrorDetail {
                message: message.to_string(),
                error_type: "OAuthException".to_string(),
                code,
                fbtrace_id: format!("trace_{}", Uuid::new_v4()),
            },
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/v\d+\.\d+/\d+/messages"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_rate_limit(&self) {
        self.expect_error(80007, "Rate limit hit").await;
    }

    pub async fn expect_invalid_token(&self) {
        let error_response = ErrorResponse {
            error: ErrorDetail {
                message: "Invalid OAuth access token".to_string(),
                error_type: "OAuthException".to_string(),
                code: 190,
                fbtrace_id: format!("trace_{}", Uuid::new_v4()),
            },
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/v\d+\.\d+/\d+/messages"))
            .respond_with(ResponseTemplate::new(401).set_body_json(&error_response))
            .mount(&self.server)
            .await;
    }

    #[must_use]
    pub fn sent_messages(&self) -> Vec<SentMessage> {
        self.sent_messages.lock().unwrap().clone()
    }

    #[must_use]
    pub fn sent_messages_to(&self, phone: &str) -> Vec<SentMessage> {
        self.sent_messages
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.to == phone)
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn last_sent_message(&self) -> Option<SentMessage> {
        self.sent_messages.lock().unwrap().last().cloned()
    }

    pub fn clear_sent_messages(&self) {
        self.sent_messages.lock().unwrap().clear();
    }

    #[must_use]
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub fn graph_api_url(&self) -> String {
        format!("http://127.0.0.1:{}/v17.0", self.port)
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn phone_number_id(&self) -> &str {
        &self.phone_number_id
    }

    #[must_use]
    pub fn business_account_id(&self) -> &str {
        &self.business_account_id
    }

    #[must_use]
    pub fn access_token(&self) -> &str {
        &self.access_token
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
        self.sent_messages.lock().unwrap().clear();
        self.received_webhooks.lock().unwrap().clear();
        self.expectations.lock().unwrap().clear();
        self.setup_default_routes().await;
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }
}
