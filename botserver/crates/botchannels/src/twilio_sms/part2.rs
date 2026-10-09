use super::*;

impl TwilioSmsChannel {
    pub fn parse_incoming_webhook(&self, params: &HashMap<String, String>) -> Result<IncomingWebhook, TwilioError> {
        let message_sid = params
            .get("MessageSid")
            .ok_or_else(|| TwilioError::ParseError("Missing MessageSid".to_string()))?
            .clone();

        let account_sid = params
            .get("AccountSid")
            .ok_or_else(|| TwilioError::ParseError("Missing AccountSid".to_string()))?
            .clone();

        let from = params
            .get("From")
            .ok_or_else(|| TwilioError::ParseError("Missing From".to_string()))?
            .clone();

        let to = params
            .get("To")
            .ok_or_else(|| TwilioError::ParseError("Missing To".to_string()))?
            .clone();

        let body = params.get("Body").cloned().unwrap_or_default();

        let num_media = params
            .get("NumMedia")
            .and_then(|s| s.parse().ok());

        let num_segments = params
            .get("NumSegments")
            .and_then(|s| s.parse().ok());

        let mut media_urls = Vec::new();
        if let Some(count) = num_media {
            for i in 0..count {
                if let Some(url) = params.get(&format!("MediaUrl{}", i)) {
                    media_urls.push(url.clone());
                }
            }
        }

        Ok(IncomingWebhook {
            message_sid,
            account_sid,
            from,
            to,
            body,
            num_media,
            num_segments,
            sms_sid: params.get("SmsSid").cloned(),
            sms_status: params.get("SmsStatus").cloned(),
            api_version: params.get("ApiVersion").cloned(),
            from_city: params.get("FromCity").cloned(),
            from_state: params.get("FromState").cloned(),
            from_zip: params.get("FromZip").cloned(),
            from_country: params.get("FromCountry").cloned(),
            to_city: params.get("ToCity").cloned(),
            to_state: params.get("ToState").cloned(),
            to_zip: params.get("ToZip").cloned(),
            to_country: params.get("ToCountry").cloned(),
            media_urls,
        })
    }

    pub fn parse_status_callback(&self, params: &HashMap<String, String>) -> Result<StatusCallback, TwilioError> {
        let message_sid = params
            .get("MessageSid")
            .ok_or_else(|| TwilioError::ParseError("Missing MessageSid".to_string()))?
            .clone();

        let message_status_str = params
            .get("MessageStatus")
            .ok_or_else(|| TwilioError::ParseError("Missing MessageStatus".to_string()))?;

        let message_status = parse_message_status(message_status_str)?;

        let account_sid = params
            .get("AccountSid")
            .ok_or_else(|| TwilioError::ParseError("Missing AccountSid".to_string()))?
            .clone();

        let from = params.get("From").cloned().unwrap_or_default();
        let to = params.get("To").cloned().unwrap_or_default();

        let error_code = params
            .get("ErrorCode")
            .and_then(|s| s.parse().ok());

        Ok(StatusCallback {
            message_sid,
            message_status,
            account_sid,
            from,
            to,
            api_version: params.get("ApiVersion").cloned(),
            error_code,
            error_message: params.get("ErrorMessage").cloned(),
        })
    }

    pub async fn handle_incoming_message(&self, webhook: IncomingWebhook) -> ConversationContext {
        let message = SmsMessage {
            sid: webhook.message_sid.clone(),
            account_sid: webhook.account_sid.clone(),
            from: webhook.from.clone(),
            to: webhook.to.clone(),
            body: webhook.body.clone(),
            status: MessageStatus::Received,
            direction: MessageDirection::Inbound,
            date_created: Some(Utc::now()),
            date_sent: Some(Utc::now()),
            date_updated: Some(Utc::now()),
            price: None,
            price_unit: None,
            error_code: None,
            error_message: None,
            num_segments: webhook.num_segments,
            num_media: webhook.num_media,
        };

        self.update_conversation(&webhook.from, &message, MessageDirection::Inbound)
            .await
    }

    pub fn generate_twiml_response(&self, message: Option<&str>, media_url: Option<&str>) -> String {
        let mut twiml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Response>");

        if let Some(msg) = message {
            if let Some(media) = media_url {
                twiml.push_str(&format!(
                    "\n  <Message>\n    <Body>{}</Body>\n    <Media>{}</Media>\n  </Message>",
                    escape_xml(msg),
                    escape_xml(media)
                ));
            } else {
                twiml.push_str(&format!(
                    "\n  <Message>{}</Message>",
                    escape_xml(msg)
                ));
            }
        }

        twiml.push_str("\n</Response>");
        twiml
    }

    pub async fn get_conversation(&self, phone_number: &str) -> Option<ConversationContext> {
        let conversations = self.conversations.read().await;
        conversations.get(phone_number).cloned()
    }

    pub async fn get_phone_numbers(&self) -> Result<Vec<TwilioPhoneNumber>, TwilioError> {
        let url = format!(
            "{}/Accounts/{}/IncomingPhoneNumbers.json",
            Self::API_BASE, self.config.account_sid
        );

        let response = self
            .http_client
            .get(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .send()
            .await
            .map_err(|e| TwilioError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error: TwilioApiError = response
                .json()
                .await
                .unwrap_or_else(|_| TwilioApiError {
                    code: 0,
                    message: "Failed to list phone numbers".to_string(),
                    more_info: None,
                    status: 500,
                });
            return Err(TwilioError::ApiError(error));
        }

        let list_response: PhoneNumberListResponse = response
            .json()
            .await
            .map_err(|e| TwilioError::ParseError(e.to_string()))?;

        let numbers = list_response
            .incoming_phone_numbers
            .into_iter()
            .map(|p| TwilioPhoneNumber {
                sid: p.sid,
                phone_number: p.phone_number,
                friendly_name: p.friendly_name,
                capabilities: PhoneCapabilities {
                    sms: p.capabilities.sms.unwrap_or(false),
                    mms: p.capabilities.mms.unwrap_or(false),
                    voice: p.capabilities.voice.unwrap_or(false),
                    fax: p.capabilities.fax.unwrap_or(false),
                },
                status: p.status,
            })
            .collect();

        Ok(numbers)
    }

    pub fn validate_webhook_signature(
        &self,
        signature: &str,
        url: &str,
        params: &HashMap<String, String>,
    ) -> bool {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;

        let mut sorted_params: Vec<(&String, &String)> = params.iter().collect();
        sorted_params.sort_by(|a, b| a.0.cmp(b.0));

        let mut data = url.to_string();
        for (key, value) in sorted_params {
            data.push_str(key);
            data.push_str(value);
        }

        let mut mac = match Hmac::<Sha1>::new_from_slice(self.config.auth_token.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };

        mac.update(data.as_bytes());
        let result = mac.finalize();
        let computed_signature = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            result.into_bytes(),
        );

        signature == computed_signature
    }

    async fn update_conversation(
        &self,
        phone_number: &str,
        message: &SmsMessage,
        direction: MessageDirection,
    ) -> ConversationContext {
        let mut conversations = self.conversations.write().await;

        let context = conversations
            .entry(phone_number.to_string())
            .or_insert_with(|| ConversationContext {
                id: Uuid::new_v4(),
                phone_number: phone_number.to_string(),
                messages: Vec::new(),
                created_at: Utc::now(),
                last_message_at: Utc::now(),
                metadata: HashMap::new(),
            });

        context.messages.push(ConversationMessage {
            sid: message.sid.clone(),
            direction,
            body: message.body.clone(),
            timestamp: Utc::now(),
            status: message.status,
        });

        context.last_message_at = Utc::now();

        context.clone()
    }

    pub(super) fn convert_response_to_message(&self, response: TwilioMessageResponse) -> SmsMessage {
        SmsMessage {
            sid: response.sid,
            account_sid: response.account_sid,
            from: response.from.unwrap_or_default(),
            to: response.to,
            body: response.body.unwrap_or_default(),
            status: parse_message_status(&response.status).unwrap_or(MessageStatus::Queued),
            direction: parse_direction(&response.direction.unwrap_or_default()),
            date_created: response.date_created.and_then(|d| DateTime::parse_from_rfc2822(&d).ok().map(|dt| dt.with_timezone(&Utc))),
            date_sent: response.date_sent.and_then(|d| DateTime::parse_from_rfc2822(&d).ok().map(|dt| dt.with_timezone(&Utc))),
            date_updated: response.date_updated.and_then(|d| DateTime::parse_from_rfc2822(&d).ok().map(|dt| dt.with_timezone(&Utc))),
            price: response.price,
            price_unit: response.price_unit,
            error_code: response.error_code,
            error_message: response.error_message,
            num_segments: response.num_segments.and_then(|s| s.parse().ok()),
            num_media: response.num_media.and_then(|s| s.parse().ok()),
        }
    }
}
