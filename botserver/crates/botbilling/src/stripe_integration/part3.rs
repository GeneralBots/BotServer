use super::*;

impl StripeClient {
    /// Attaches an existing PaymentMethod (e.g. created by a SetupIntent) to
    /// a customer.
    pub async fn attach_payment_method(
        &self,
        customer_id: &str,
        payment_method_id: &str,
    ) -> Result<StripePaymentMethod, StripeError> {
        let form: Vec<(String, String)> =
            vec![("customer".to_string(), customer_id.to_string())];

        let response = self
            .client
            .post(format!("{}/payment_methods/{}/attach", Self::API_BASE, payment_method_id))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Detaches a PaymentMethod from its customer (removes it as a saved card).
    pub async fn detach_payment_method(
        &self,
        payment_method_id: &str,
    ) -> Result<StripePaymentMethod, StripeError> {
        let response = self
            .client
            .post(format!("{}/payment_methods/{}/detach", Self::API_BASE, payment_method_id))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Sets the default payment method used for future invoices on a customer.
    pub async fn set_default_payment_method(
        &self,
        customer_id: &str,
        payment_method_id: &str,
    ) -> Result<StripeCustomer, StripeError> {
        let form: Vec<(String, String)> = vec![(
            "invoice_settings[default_payment_method]".to_string(),
            payment_method_id.to_string(),
        )];

        let response = self
            .client
            .post(format!("{}/customers/{}", Self::API_BASE, customer_id))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn get_payment_methods(&self, customer_id: &str) -> Result<Vec<StripePaymentMethod>, StripeError> {
        let response = self
            .client
            .get(format!("{}/payment_methods", Self::API_BASE))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .query(&[("customer", customer_id), ("type", "card")])
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        #[derive(Deserialize)]
        struct PaymentMethodList {
            data: Vec<StripePaymentMethod>,
        }

        let list: PaymentMethodList = self.handle_response(response).await?;
        Ok(list.data)
    }

    pub fn verify_webhook_signature(&self, payload: &str, signature: &str) -> Result<StripeWebhookEvent, StripeError> {
        let webhook_secret = self
            .webhook_secret
            .as_ref()
            .ok_or(StripeError::NotConfigured)?;

        let parts: HashMap<&str, &str> = signature
            .split(',')
            .filter_map(|part| {
                let mut split = part.split('=');
                Some((split.next()?, split.next()?))
            })
            .collect();

        let timestamp = parts
            .get("t")
            .ok_or_else(|| StripeError::InvalidWebhook("Missing timestamp".to_string()))?;

        let received_sig = parts
            .get("v1")
            .ok_or_else(|| StripeError::InvalidWebhook("Missing signature".to_string()))?;

        let signed_payload = format!("{timestamp}.{payload}");

        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
            .map_err(|_| StripeError::InvalidWebhook("Invalid webhook secret".to_string()))?;

        mac.update(signed_payload.as_bytes());

        let expected_sig = hex::encode(mac.finalize().into_bytes());

        if expected_sig != *received_sig {
            return Err(StripeError::InvalidWebhook("Signature mismatch".to_string()));
        }

        let timestamp_i64: i64 = timestamp
            .parse()
            .map_err(|_| StripeError::InvalidWebhook("Invalid timestamp".to_string()))?;

        let now = chrono::Utc::now().timestamp();
        let tolerance = 300;

        if (now - timestamp_i64).abs() > tolerance {
            return Err(StripeError::InvalidWebhook("Timestamp too old".to_string()));
        }

        serde_json::from_str(payload).map_err(|e| StripeError::ParseError(e.to_string()))
    }

    pub fn parse_webhook_event(&self, event: &StripeWebhookEvent) -> Result<WebhookEventType, StripeError> {
        match event.event_type.as_str() {
            "customer.subscription.created" => {
                let subscription: StripeSubscription = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::SubscriptionCreated(subscription))
            }
            "customer.subscription.updated" => {
                let subscription: StripeSubscription = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::SubscriptionUpdated(subscription))
            }
            "customer.subscription.deleted" => {
                let subscription: StripeSubscription = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::SubscriptionCanceled(subscription))
            }
            "invoice.paid" => {
                let invoice: StripeInvoice = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::InvoicePaid(invoice))
            }
            "invoice.payment_failed" => {
                let invoice: StripeInvoice = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::InvoicePaymentFailed(invoice))
            }
            "checkout.session.completed" => {
                let session: StripeCheckoutSession = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::CheckoutCompleted(session))
            }
            "payment_method.attached" | "customer.source.created" => {
                let pm: StripePaymentMethod = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::PaymentMethodAttached(pm))
            }
            "payment_method.updated" | "customer.source.updated" => {
                let pm: StripePaymentMethod = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::PaymentMethodUpdated(pm))
            }
            "payment_method.detached" | "customer.source.deleted" => {
                let pm: StripePaymentMethod = serde_json::from_value(event.data.object.clone())
                    .map_err(|e| StripeError::ParseError(e.to_string()))?;
                Ok(WebhookEventType::PaymentMethodDetached(pm))
            }
            _ => Ok(WebhookEventType::Unknown(event.event_type.clone())),
        }
    }

    pub(crate) async fn handle_response<T: serde::de::DeserializeOwned>(&self, response: reqwest::Response) -> Result<T, StripeError> {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        if !status.is_success() {
            #[derive(Deserialize)]
            struct StripeApiError {
                error: StripeApiErrorDetail,
            }

            #[derive(Deserialize)]
            struct StripeApiErrorDetail {
                message: String,
            }

            if let Ok(error) = serde_json::from_str::<StripeApiError>(&body) {
                return Err(StripeError::ApiError(error.error.message));
            }

            return Err(StripeError::ApiError(format!("HTTP {}: {}", status, body)));
        }

        serde_json::from_str(&body).map_err(|e| StripeError::ParseError(e.to_string()))
    }
}
