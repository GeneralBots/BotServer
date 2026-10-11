use super::*;

impl SubscriptionLifecycleService {
    pub async fn reactivate_subscription(
        &self,
        subscription_id: Uuid,
    ) -> Result<Subscription, LifecycleError> {
        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions
            .get_mut(&subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        if subscription.status != SubscriptionStatus::Canceled {
            return Err(LifecycleError::InvalidOperation(
                "Subscription is not cancelled".to_string(),
            ));
        }

        let now = Utc::now();
        subscription.status = SubscriptionStatus::Active;
        subscription.current_period_start = now;
        subscription.current_period_end = now + Duration::days(30);
        subscription.updated_at = now;

        let result = subscription.clone();
        let org_id = subscription.organization_id;
        let plan_id = subscription.plan_id.clone();

        drop(subscriptions);

        self.record_event(
            subscription_id,
            org_id,
            LifecycleEventType::Resumed,
            None,
            Some(plan_id),
            HashMap::new(),
        )
        .await;

        Ok(result)
    }

    pub async fn pause_subscription(&self, subscription_id: Uuid) -> Result<Subscription, LifecycleError> {
        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions
            .get_mut(&subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        if subscription.status != SubscriptionStatus::Active {
            return Err(LifecycleError::InvalidOperation(
                "Only active subscriptions can be paused".to_string(),
            ));
        }

        subscription.status = SubscriptionStatus::Paused;
        subscription.updated_at = Utc::now();

        let result = subscription.clone();
        let org_id = subscription.organization_id;

        drop(subscriptions);

        self.record_event(
            subscription_id,
            org_id,
            LifecycleEventType::Paused,
            None,
            None,
            HashMap::new(),
        )
        .await;

        Ok(result)
    }

    pub async fn resume_subscription(&self, subscription_id: Uuid) -> Result<Subscription, LifecycleError> {
        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions
            .get_mut(&subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        if subscription.status != SubscriptionStatus::Paused {
            return Err(LifecycleError::InvalidOperation(
                "Only paused subscriptions can be resumed".to_string(),
            ));
        }

        subscription.status = SubscriptionStatus::Active;
        subscription.updated_at = Utc::now();

        let result = subscription.clone();
        let org_id = subscription.organization_id;

        drop(subscriptions);

        self.record_event(
            subscription_id,
            org_id,
            LifecycleEventType::Resumed,
            None,
            None,
            HashMap::new(),
        )
        .await;

        Ok(result)
    }

    pub async fn renew_subscription(&self, subscription_id: Uuid) -> Result<Subscription, LifecycleError> {
        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions
            .get_mut(&subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        let now = Utc::now();
        subscription.current_period_start = now;
        subscription.current_period_end = now + Duration::days(30);
        subscription.status = SubscriptionStatus::Active;
        subscription.updated_at = now;

        let result = subscription.clone();
        let org_id = subscription.organization_id;

        drop(subscriptions);

        self.record_event(
            subscription_id,
            org_id,
            LifecycleEventType::Renewed,
            None,
            None,
            HashMap::new(),
        )
        .await;

        Ok(result)
    }

    pub async fn handle_payment_failure(&self, subscription_id: Uuid) -> Result<(), LifecycleError> {
        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions
            .get_mut(&subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        subscription.status = SubscriptionStatus::PastDue;
        subscription.updated_at = Utc::now();

        let org_id = subscription.organization_id;

        drop(subscriptions);

        self.record_event(
            subscription_id,
            org_id,
            LifecycleEventType::PaymentFailed,
            None,
            None,
            HashMap::new(),
        )
        .await;

        Ok(())
    }

    pub async fn handle_payment_recovery(&self, subscription_id: Uuid) -> Result<Subscription, LifecycleError> {
        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions
            .get_mut(&subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        subscription.status = SubscriptionStatus::Active;
        subscription.updated_at = Utc::now();

        let result = subscription.clone();
        let org_id = subscription.organization_id;

        drop(subscriptions);

        self.record_event(
            subscription_id,
            org_id,
            LifecycleEventType::PaymentRecovered,
            None,
            None,
            HashMap::new(),
        )
        .await;

        Ok(result)
    }

    pub async fn accept_retention_offer(&self, offer_id: Uuid) -> Result<Subscription, LifecycleError> {
        let mut offers = self.retention_offers.write().await;
        let offer = offers
            .get_mut(&offer_id)
            .ok_or(LifecycleError::OfferNotFound)?;

        if offer.expires_at < Utc::now() {
            return Err(LifecycleError::OfferExpired);
        }

        if offer.accepted {
            return Err(LifecycleError::OfferAlreadyAccepted);
        }

        offer.accepted = true;
        let subscription_id = offer.subscription_id;

        drop(offers);

        let subscriptions = self.subscriptions.read().await;
        let subscription = subscriptions
            .get(&subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?
            .clone();

        Ok(subscription)
    }

    pub async fn get_events(&self, subscription_id: Uuid) -> Vec<SubscriptionLifecycleEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.subscription_id == subscription_id)
            .cloned()
            .collect()
    }

    pub async fn get_pending_change(&self, subscription_id: Uuid) -> Option<SubscriptionChange> {
        let pending = self.pending_changes.read().await;
        pending.get(&subscription_id).cloned()
    }

    pub async fn apply_pending_changes(&self) -> Vec<SubscriptionChange> {
        let now = Utc::now();
        let mut applied = Vec::new();

        let pending = self.pending_changes.read().await;
        let due_changes: Vec<SubscriptionChange> = pending
            .values()
            .filter(|c| c.effective_date <= now && c.status == ChangeStatus::Scheduled)
            .cloned()
            .collect();
        drop(pending);

        for change in due_changes {
            let mut subscriptions = self.subscriptions.write().await;
            if let Some(subscription) = subscriptions.get_mut(&change.subscription_id) {
                subscription.plan_id = change.to_plan.clone();
                subscription.updated_at = now;

                let mut pending = self.pending_changes.write().await;
                if let Some(pending_change) = pending.get_mut(&change.subscription_id) {
                    pending_change.status = ChangeStatus::Applied;
                }

                applied.push(change.clone());
            }
        }

        applied
    }

    pub(crate) async fn create_retention_offer(
        &self,
        subscription_id: Uuid,
    ) -> Result<RetentionOffer, LifecycleError> {
        let subscriptions = self.subscriptions.read().await;
        let _subscription = subscriptions
            .get(&subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        let offer = RetentionOffer {
            id: Uuid::new_v4(),
            subscription_id,
            offer_type: RetentionOfferType::Discount,
            discount_percent: Some(20),
            free_months: None,
            expires_at: Utc::now() + Duration::days(7),
            accepted: false,
        };

        drop(subscriptions);

        let mut offers = self.retention_offers.write().await;
        offers.insert(offer.id, offer.clone());

        Ok(offer)
    }
}
