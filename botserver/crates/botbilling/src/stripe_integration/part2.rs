use super::*;

impl StripeClient {
    pub fn new(api_key: String, webhook_secret: Option<String>) -> Self {
        Self {
            api_key,
            webhook_secret,
            client: reqwest::Client::new(),
        }
    }

    pub async fn create_customer(&self, params: CreateCustomerParams) -> Result<StripeCustomer, StripeError> {
        let mut form: Vec<(String, String)> = vec![("email".to_string(), params.email)];

        if let Some(name) = params.name {
            form.push(("name".to_string(), name));
        }

        form.push(("metadata[organization_id]".to_string(), params.organization_id.to_string()));

        for (key, value) in params.metadata {
            form.push((format!("metadata[{key}]"), value));
        }

        let response = self
            .client
            .post(format!("{}/customers", Self::API_BASE))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn get_customer(&self, customer_id: &str) -> Result<StripeCustomer, StripeError> {
        let response = self
            .client
            .get(format!("{}/customers/{}", Self::API_BASE, customer_id))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn create_checkout_session(
        &self,
        params: CreateCheckoutSessionParams,
    ) -> Result<StripeCheckoutSession, StripeError> {
        let mut form: Vec<(String, String)> = vec![
            ("customer".to_string(), params.customer_id),
            ("mode".to_string(), "subscription".to_string()),
            ("success_url".to_string(), params.success_url),
            ("cancel_url".to_string(), params.cancel_url),
            ("line_items[0][price]".to_string(), params.price_id),
            ("line_items[0][quantity]".to_string(), "1".to_string()),
        ];

        if let Some(days) = params.trial_days {
            form.push(("subscription_data[trial_period_days]".to_string(), days.to_string()));
        }

        for (key, value) in params.metadata {
            form.push((format!("metadata[{key}]"), value));
        }

        let response = self
            .client
            .post(format!("{}/checkout/sessions", Self::API_BASE))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn retrieve_checkout_session(
        &self,
        session_id: &str,
    ) -> Result<StripeCheckoutSession, StripeError> {
        let response = self
            .client
            .get(format!("{}/checkout/sessions/{}", Self::API_BASE, session_id))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn create_portal_session(
        &self,
        params: CreatePortalSessionParams,
    ) -> Result<StripeBillingPortalSession, StripeError> {
        let form: Vec<(String, String)> = vec![
            ("customer".to_string(), params.customer_id),
            ("return_url".to_string(), params.return_url),
        ];

        let response = self
            .client
            .post(format!("{}/billing_portal/sessions", Self::API_BASE))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn get_subscription(&self, subscription_id: &str) -> Result<StripeSubscription, StripeError> {
        let response = self
            .client
            .get(format!("{}/subscriptions/{}", Self::API_BASE, subscription_id))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn cancel_subscription(&self, subscription_id: &str, at_period_end: bool) -> Result<StripeSubscription, StripeError> {
        let form: Vec<(String, String)> = if at_period_end {
            vec![("cancel_at_period_end".to_string(), "true".to_string())]
        } else {
            vec![]
        };

        let url = format!("{}/subscriptions/{}", Self::API_BASE, subscription_id);

        let request = if at_period_end {
            self.client.post(&url).form(&form)
        } else {
            self.client.delete(&url)
        };

        let response = request
            .basic_auth(&self.api_key, Option::<&str>::None)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn update_subscription(
        &self,
        subscription_id: &str,
        new_price_id: &str,
    ) -> Result<StripeSubscription, StripeError> {
        let subscription = self.get_subscription(subscription_id).await?;

        let item_id = subscription
            .items
            .data
            .first()
            .map(|item| item.id.clone())
            .ok_or_else(|| StripeError::ApiError("No subscription items found".to_string()))?;

        let form: Vec<(String, String)> = vec![
            ("items[0][id]".to_string(), item_id),
            ("items[0][price]".to_string(), new_price_id.to_string()),
            ("proration_behavior".to_string(), "create_prorations".to_string()),
        ];

        let response = self
            .client
            .post(format!("{}/subscriptions/{}", Self::API_BASE, subscription_id))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn list_invoices(&self, customer_id: &str, limit: u32) -> Result<Vec<StripeInvoice>, StripeError> {
        let response = self
            .client
            .get(format!("{}/invoices", Self::API_BASE))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .query(&[("customer", customer_id), ("limit", &limit.to_string())])
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        #[derive(Deserialize)]
        struct InvoiceList {
            data: Vec<StripeInvoice>,
        }

        let list: InvoiceList = self.handle_response(response).await?;
        Ok(list.data)
    }

    /// Finds the most recent Stripe Customer for an email address.
    pub async fn find_customer_by_email(
        &self,
        email: &str,
    ) -> Result<Option<StripeCustomer>, StripeError> {
        let response = self
            .client
            .get(format!("{}/customers", Self::API_BASE))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .query(&[("email", email), ("limit", "1")])
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        #[derive(Deserialize)]
        struct CustomerList {
            data: Vec<StripeCustomer>,
        }

        let list: CustomerList = self.handle_response(response).await?;
        Ok(list.data.into_iter().next())
    }

    /// Creates a SetupIntent to collect a card via Stripe hosted fields /
    /// Elements. The returned `client_secret` is used client-side; the card
    /// number never transits our servers.
    pub async fn create_setup_intent(
        &self,
        params: CreateSetupIntentParams,
    ) -> Result<StripeSetupIntent, StripeError> {
        let mut form: Vec<(String, String)> = vec![
            ("customer".to_string(), params.customer_id),
            ("usage".to_string(), params.usage),
            ("payment_method_types[0]".to_string(), "card".to_string()),
        ];
        // Preserve backwards compatibility with clients that relied on the
        // default behavior of accepting every supported payment method.
        form.push(("automatic_payment_methods[enabled]".to_string(), "true".to_string()));

        let response = self
            .client
            .post(format!("{}/setup_intents", Self::API_BASE))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    pub async fn retrieve_setup_intent(
        &self,
        setup_intent_id: &str,
    ) -> Result<StripeSetupIntent, StripeError> {
        let response = self
            .client
            .get(format!("{}/setup_intents/{}", Self::API_BASE, setup_intent_id))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Creates a fully hosted Checkout session in `setup` mode so users can
    /// add a card on Stripe's domain — no Elements JS or raw card fields in
    /// our HTML (PCI SAQ-A).
    pub async fn create_setup_checkout_session(
        &self,
        params: CreateSetupCheckoutSessionParams,
    ) -> Result<StripeCheckoutSession, StripeError> {
        let mut form: Vec<(String, String)> = vec![
            ("customer".to_string(), params.customer_id),
            ("mode".to_string(), "setup".to_string()),
            ("success_url".to_string(), params.success_url),
            ("cancel_url".to_string(), params.cancel_url),
        ];

        for (key, value) in params.metadata {
            form.push((format!("metadata[{key}]"), value));
        }

        let response = self
            .client
            .post(format!("{}/checkout/sessions", Self::API_BASE))
            .basic_auth(&self.api_key, Option::<&str>::None)
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::NetworkError(e.to_string()))?;

        self.handle_response(response).await
    }
}
