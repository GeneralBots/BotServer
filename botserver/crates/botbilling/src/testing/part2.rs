use super::*;

impl MockStripeClient {
    pub async fn cancel_subscription(
        &self,
        subscription_id: &str,
        immediately: bool,
    ) -> Result<MockStripeSubscription, MockStripeError> {
        self.check_failure().await?;

        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions.get_mut(subscription_id).ok_or_else(|| {
            MockStripeError::NotFound(format!("Subscription {subscription_id} not found"))
        })?;

        if immediately {
            subscription.status = SubscriptionStatus::Canceled;
            subscription.canceled_at = Some(Utc::now().timestamp());
        } else {
            subscription.cancel_at_period_end = true;
        }

        let result = subscription.clone();
        drop(subscriptions);

        self.emit_event("customer.subscription.deleted", &result).await;

        Ok(result)
    }

    pub async fn create_invoice(
        &self,
        customer_id: &str,
        subscription_id: Option<&str>,
        amount: i64,
    ) -> Result<MockStripeInvoice, MockStripeError> {
        self.check_failure().await?;

        let customers = self.customers.read().await;
        if !customers.contains_key(customer_id) {
            return Err(MockStripeError::NotFound(format!(
                "Customer {customer_id} not found"
            )));
        }
        drop(customers);

        let invoice_id = format!("in_{}", generate_stripe_id());

        let invoice = MockStripeInvoice {
            id: invoice_id.clone(),
            customer: customer_id.to_string(),
            subscription: subscription_id.map(|s| s.to_string()),
            status: InvoiceStatus::Draft,
            amount_due: amount,
            amount_paid: 0,
            amount_remaining: amount,
            currency: "usd".to_string(),
            created: Utc::now().timestamp(),
            due_date: Some((Utc::now() + Duration::days(30)).timestamp()),
            paid: false,
            lines: MockInvoiceLines {
                data: vec![MockInvoiceLine {
                    id: format!("il_{}", generate_stripe_id()),
                    amount,
                    currency: "usd".to_string(),
                    description: Some("Subscription".to_string()),
                    quantity: 1,
                    price: MockPrice {
                        id: "price_auto".to_string(),
                        product: "prod_auto".to_string(),
                        unit_amount: amount,
                        currency: "usd".to_string(),
                        recurring: None,
                    },
                }],
            },
            hosted_invoice_url: Some(format!("https://invoice.stripe.com/i/{invoice_id}")),
            invoice_pdf: Some(format!("https://invoice.stripe.com/i/{invoice_id}/pdf")),
        };

        let mut invoices = self.invoices.write().await;
        invoices.insert(invoice.id.clone(), invoice.clone());

        Ok(invoice)
    }

    pub async fn get_invoice(&self, invoice_id: &str) -> Result<MockStripeInvoice, MockStripeError> {
        self.check_failure().await?;

        let invoices = self.invoices.read().await;
        invoices
            .get(invoice_id)
            .cloned()
            .ok_or_else(|| MockStripeError::NotFound(format!("Invoice {invoice_id} not found")))
    }

    pub async fn finalize_invoice(&self, invoice_id: &str) -> Result<MockStripeInvoice, MockStripeError> {
        self.check_failure().await?;

        let mut invoices = self.invoices.write().await;
        let invoice = invoices
            .get_mut(invoice_id)
            .ok_or_else(|| MockStripeError::NotFound(format!("Invoice {invoice_id} not found")))?;

        invoice.status = InvoiceStatus::Open;

        let result = invoice.clone();
        drop(invoices);

        self.emit_event("invoice.finalized", &result).await;

        Ok(result)
    }

    pub async fn pay_invoice(&self, invoice_id: &str) -> Result<MockStripeInvoice, MockStripeError> {
        self.check_failure().await?;

        let mut invoices = self.invoices.write().await;
        let invoice = invoices
            .get_mut(invoice_id)
            .ok_or_else(|| MockStripeError::NotFound(format!("Invoice {invoice_id} not found")))?;

        invoice.status = InvoiceStatus::Paid;
        invoice.paid = true;
        invoice.amount_paid = invoice.amount_due;
        invoice.amount_remaining = 0;

        let result = invoice.clone();
        drop(invoices);

        self.emit_event("invoice.paid", &result).await;

        Ok(result)
    }

    pub async fn void_invoice(&self, invoice_id: &str) -> Result<MockStripeInvoice, MockStripeError> {
        self.check_failure().await?;

        let mut invoices = self.invoices.write().await;
        let invoice = invoices
            .get_mut(invoice_id)
            .ok_or_else(|| MockStripeError::NotFound(format!("Invoice {invoice_id} not found")))?;

        invoice.status = InvoiceStatus::Void;

        let result = invoice.clone();
        drop(invoices);

        self.emit_event("invoice.voided", &result).await;

        Ok(result)
    }

    pub async fn create_payment_intent(
        &self,
        amount: i64,
        currency: &str,
        customer_id: Option<&str>,
    ) -> Result<MockStripePaymentIntent, MockStripeError> {
        self.check_failure().await?;

        let payment_intent = MockStripePaymentIntent {
            id: format!("pi_{}", generate_stripe_id()),
            amount,
            currency: currency.to_string(),
            status: PaymentIntentStatus::RequiresPaymentMethod,
            customer: customer_id.map(|s| s.to_string()),
            payment_method: None,
            client_secret: format!("pi_{}_secret_{}", generate_stripe_id(), generate_stripe_id()),
            created: Utc::now().timestamp(),
        };

        let mut payment_intents = self.payment_intents.write().await;
        payment_intents.insert(payment_intent.id.clone(), payment_intent.clone());

        Ok(payment_intent)
    }

    pub async fn confirm_payment_intent(
        &self,
        payment_intent_id: &str,
        payment_method_id: &str,
    ) -> Result<MockStripePaymentIntent, MockStripeError> {
        self.check_failure().await?;

        let mut payment_intents = self.payment_intents.write().await;
        let pi = payment_intents.get_mut(payment_intent_id).ok_or_else(|| {
            MockStripeError::NotFound(format!("PaymentIntent {payment_intent_id} not found"))
        })?;

        pi.payment_method = Some(payment_method_id.to_string());
        pi.status = PaymentIntentStatus::Succeeded;

        let result = pi.clone();
        drop(payment_intents);

        self.emit_event("payment_intent.succeeded", &result).await;

        Ok(result)
    }

    pub async fn create_payment_method(
        &self,
        card_brand: &str,
        last4: &str,
        exp_month: u32,
        exp_year: u32,
    ) -> Result<MockStripePaymentMethod, MockStripeError> {
        self.check_failure().await?;

        let pm = MockStripePaymentMethod {
            id: format!("pm_{}", generate_stripe_id()),
            customer: None,
            payment_type: "card".to_string(),
            card: Some(MockCard {
                brand: card_brand.to_string(),
                last4: last4.to_string(),
                exp_month,
                exp_year,
                funding: "credit".to_string(),
            }),
            created: Utc::now().timestamp(),
        };

        let mut payment_methods = self.payment_methods.write().await;
        payment_methods.insert(pm.id.clone(), pm.clone());

        Ok(pm)
    }

    pub async fn attach_payment_method(
        &self,
        payment_method_id: &str,
        customer_id: &str,
    ) -> Result<MockStripePaymentMethod, MockStripeError> {
        self.check_failure().await?;

        let customers = self.customers.read().await;
        if !customers.contains_key(customer_id) {
            return Err(MockStripeError::NotFound(format!(
                "Customer {customer_id} not found"
            )));
        }
        drop(customers);

        let mut payment_methods = self.payment_methods.write().await;
        let pm = payment_methods.get_mut(payment_method_id).ok_or_else(|| {
            MockStripeError::NotFound(format!("PaymentMethod {payment_method_id} not found"))
        })?;

        pm.customer = Some(customer_id.to_string());

        Ok(pm.clone())
    }

    pub async fn get_events(&self, limit: usize) -> Vec<MockStripeEvent> {
        let events = self.events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }

    pub(crate) async fn emit_event<T: Serialize>(&self, event_type: &str, data: &T) {
        let event = MockStripeEvent {
            id: format!("evt_{}", generate_stripe_id()),
            event_type: event_type.to_string(),
            created: Utc::now().timestamp(),
            data: MockEventData {
                object: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
            },
            livemode: false,
        };

        let mut events = self.events.write().await;
        events.push(event);

        if events.len() > 1000 {
            events.drain(0..100);
        }
    }
}
