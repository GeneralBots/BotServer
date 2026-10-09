use super::*;

impl StripeWebhookHandler {
    pub fn new(client: StripeClient) -> Self {
        Self { client }
    }

    pub async fn handle_webhook(&self, payload: &str, signature: &str) -> Result<WebhookAction, StripeError> {
        let event = self.client.verify_webhook_signature(payload, signature)?;
        let event_type = self.client.parse_webhook_event(&event)?;

        match event_type {
            WebhookEventType::SubscriptionCreated(sub) => {
                Ok(WebhookAction::ActivateSubscription {
                    stripe_subscription_id: sub.id,
                    stripe_customer_id: sub.customer,
                    status: sub.status,
                    period_end: sub.current_period_end,
                })
            }
            WebhookEventType::SubscriptionUpdated(sub) => {
                Ok(WebhookAction::UpdateSubscription {
                    stripe_subscription_id: sub.id,
                    status: sub.status,
                    cancel_at_period_end: sub.cancel_at_period_end,
                    period_end: sub.current_period_end,
                })
            }
            WebhookEventType::SubscriptionCanceled(sub) => {
                Ok(WebhookAction::CancelSubscription {
                    stripe_subscription_id: sub.id,
                })
            }
            WebhookEventType::InvoicePaid(invoice) => {
                Ok(WebhookAction::RecordPayment {
                    stripe_invoice_id: invoice.id,
                    amount: invoice.amount_paid,
                    currency: invoice.currency,
                })
            }
            WebhookEventType::InvoicePaymentFailed(invoice) => {
                Ok(WebhookAction::PaymentFailed {
                    stripe_invoice_id: invoice.id,
                    stripe_customer_id: invoice.customer,
                })
            }
            WebhookEventType::CheckoutCompleted(session) => {
                Ok(WebhookAction::CheckoutCompleted {
                    stripe_customer_id: session.customer,
                    stripe_subscription_id: session.subscription,
                })
            }
            WebhookEventType::PaymentMethodAttached(pm) => {
                Ok(WebhookAction::PaymentMethodSync {
                    pm: pm.clone(),
                    detach: false,
                })
            }
            WebhookEventType::PaymentMethodUpdated(pm) => {
                Ok(WebhookAction::PaymentMethodSync {
                    pm: pm.clone(),
                    detach: false,
                })
            }
            WebhookEventType::PaymentMethodDetached(pm) => {
                Ok(WebhookAction::PaymentMethodSync {
                    pm: pm.clone(),
                    detach: true,
                })
            }
            WebhookEventType::Unknown(event_type) => {
                tracing::debug!("Unhandled Stripe webhook event: {}", event_type);
                Ok(WebhookAction::None)
            }
        }
    }
}
