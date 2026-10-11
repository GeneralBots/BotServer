use super::*;

impl MockStripeClient {
    pub fn new() -> Self {
        let client = Self {
            customers: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            invoices: Arc::new(RwLock::new(HashMap::new())),
            payment_intents: Arc::new(RwLock::new(HashMap::new())),
            payment_methods: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            prices: Arc::new(RwLock::new(HashMap::new())),
            should_fail: Arc::new(RwLock::new(false)),
            failure_code: Arc::new(RwLock::new(None)),
        };

        tokio::spawn({
            let prices = client.prices.clone();
            async move {
                let mut p = prices.write().await;
                p.insert(
                    "price_free".to_string(),
                    MockPrice {
                        id: "price_free".to_string(),
                        product: "prod_free".to_string(),
                        unit_amount: 0,
                        currency: "usd".to_string(),
                        recurring: Some(MockRecurring {
                            interval: "month".to_string(),
                            interval_count: 1,
                        }),
                    },
                );
                p.insert(
                    "price_starter".to_string(),
                    MockPrice {
                        id: "price_starter".to_string(),
                        product: "prod_starter".to_string(),
                        unit_amount: 2900,
                        currency: "usd".to_string(),
                        recurring: Some(MockRecurring {
                            interval: "month".to_string(),
                            interval_count: 1,
                        }),
                    },
                );
                p.insert(
                    "price_pro".to_string(),
                    MockPrice {
                        id: "price_pro".to_string(),
                        product: "prod_pro".to_string(),
                        unit_amount: 4900,
                        currency: "usd".to_string(),
                        recurring: Some(MockRecurring {
                            interval: "month".to_string(),
                            interval_count: 1,
                        }),
                    },
                );
                p.insert(
                    "price_enterprise".to_string(),
                    MockPrice {
                        id: "price_enterprise".to_string(),
                        product: "prod_enterprise".to_string(),
                        unit_amount: 19900,
                        currency: "usd".to_string(),
                        recurring: Some(MockRecurring {
                            interval: "month".to_string(),
                            interval_count: 1,
                        }),
                    },
                );
            }
        });

        client
    }

    pub async fn set_should_fail(&self, should_fail: bool, code: Option<String>) {
        let mut fail = self.should_fail.write().await;
        *fail = should_fail;
        let mut failure_code = self.failure_code.write().await;
        *failure_code = code;
    }

    pub(crate) async fn check_failure(&self) -> Result<(), MockStripeError> {
        let should_fail = *self.should_fail.read().await;
        if should_fail {
            let code = self.failure_code.read().await.clone();
            return Err(MockStripeError::ApiError(
                code.unwrap_or_else(|| "card_declined".to_string()),
            ));
        }
        Ok(())
    }

    pub async fn create_customer(
        &self,
        email: &str,
        name: Option<&str>,
        metadata: HashMap<String, String>,
    ) -> Result<MockStripeCustomer, MockStripeError> {
        self.check_failure().await?;

        let customer = MockStripeCustomer {
            id: format!("cus_{}", generate_stripe_id()),
            email: email.to_string(),
            name: name.map(|s| s.to_string()),
            metadata,
            created: Utc::now().timestamp(),
            default_source: None,
            invoice_settings: MockInvoiceSettings {
                default_payment_method: None,
            },
        };

        let mut customers = self.customers.write().await;
        customers.insert(customer.id.clone(), customer.clone());

        Ok(customer)
    }

    pub async fn get_customer(&self, customer_id: &str) -> Result<MockStripeCustomer, MockStripeError> {
        self.check_failure().await?;

        let customers = self.customers.read().await;
        customers
            .get(customer_id)
            .cloned()
            .ok_or_else(|| MockStripeError::NotFound(format!("Customer {customer_id} not found")))
    }

    pub async fn update_customer(
        &self,
        customer_id: &str,
        email: Option<&str>,
        name: Option<&str>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<MockStripeCustomer, MockStripeError> {
        self.check_failure().await?;

        let mut customers = self.customers.write().await;
        let customer = customers
            .get_mut(customer_id)
            .ok_or_else(|| MockStripeError::NotFound(format!("Customer {customer_id} not found")))?;

        if let Some(e) = email {
            customer.email = e.to_string();
        }
        if let Some(n) = name {
            customer.name = Some(n.to_string());
        }
        if let Some(m) = metadata {
            customer.metadata = m;
        }

        Ok(customer.clone())
    }

    pub async fn delete_customer(&self, customer_id: &str) -> Result<(), MockStripeError> {
        self.check_failure().await?;

        let mut customers = self.customers.write().await;
        customers
            .remove(customer_id)
            .ok_or_else(|| MockStripeError::NotFound(format!("Customer {customer_id} not found")))?;

        Ok(())
    }

    pub async fn create_subscription(
        &self,
        customer_id: &str,
        price_id: &str,
        metadata: HashMap<String, String>,
        trial_days: Option<u32>,
    ) -> Result<MockStripeSubscription, MockStripeError> {
        self.check_failure().await?;

        let customers = self.customers.read().await;
        if !customers.contains_key(customer_id) {
            return Err(MockStripeError::NotFound(format!(
                "Customer {customer_id} not found"
            )));
        }
        drop(customers);

        let prices = self.prices.read().await;
        let price = prices
            .get(price_id)
            .cloned()
            .ok_or_else(|| MockStripeError::NotFound(format!("Price {price_id} not found")))?;
        drop(prices);

        let now = Utc::now();
        let (trial_start, trial_end, status) = if let Some(days) = trial_days {
            let ts = now.timestamp();
            let te = (now + Duration::days(days as i64)).timestamp();
            (Some(ts), Some(te), SubscriptionStatus::Trialing)
        } else {
            (None, None, SubscriptionStatus::Active)
        };

        let period_end = now + Duration::days(30);

        let subscription = MockStripeSubscription {
            id: format!("sub_{}", generate_stripe_id()),
            customer: customer_id.to_string(),
            status,
            current_period_start: now.timestamp(),
            current_period_end: period_end.timestamp(),
            items: MockSubscriptionItems {
                data: vec![MockSubscriptionItem {
                    id: format!("si_{}", generate_stripe_id()),
                    price,
                    quantity: 1,
                }],
            },
            metadata,
            cancel_at_period_end: false,
            canceled_at: None,
            trial_start,
            trial_end,
        };

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(subscription.id.clone(), subscription.clone());

        self.emit_event("customer.subscription.created", &subscription).await;

        Ok(subscription)
    }

    pub async fn get_subscription(
        &self,
        subscription_id: &str,
    ) -> Result<MockStripeSubscription, MockStripeError> {
        self.check_failure().await?;

        let subscriptions = self.subscriptions.read().await;
        subscriptions
            .get(subscription_id)
            .cloned()
            .ok_or_else(|| MockStripeError::NotFound(format!("Subscription {subscription_id} not found")))
    }

    pub async fn update_subscription(
        &self,
        subscription_id: &str,
        price_id: Option<&str>,
        cancel_at_period_end: Option<bool>,
    ) -> Result<MockStripeSubscription, MockStripeError> {
        self.check_failure().await?;

        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions.get_mut(subscription_id).ok_or_else(|| {
            MockStripeError::NotFound(format!("Subscription {subscription_id} not found"))
        })?;

        if let Some(pid) = price_id {
            let prices = self.prices.read().await;
            let price = prices
                .get(pid)
                .cloned()
                .ok_or_else(|| MockStripeError::NotFound(format!("Price {pid} not found")))?;

            subscription.items.data[0].price = price;
        }

        if let Some(cancel) = cancel_at_period_end {
            subscription.cancel_at_period_end = cancel;
        }

        let result = subscription.clone();
        drop(subscriptions);

        self.emit_event("customer.subscription.updated", &result).await;

        Ok(result)
    }
}
