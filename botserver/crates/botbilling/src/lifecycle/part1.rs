use super::*;

impl SubscriptionLifecycleService {
    pub fn new(plans: HashMap<String, PlanConfig>) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            pending_changes: Arc::new(RwLock::new(HashMap::new())),
            retention_offers: Arc::new(RwLock::new(HashMap::new())),
            plans: Arc::new(RwLock::new(plans)),
        }
    }

    pub async fn create_subscription(
        &self,
        request: CreateSubscriptionRequest,
    ) -> Result<Subscription, LifecycleError> {
        let plans = self.plans.read().await;
        let plan = plans
            .get(&request.plan_id)
            .ok_or_else(|| LifecycleError::PlanNotFound(request.plan_id.clone()))?;

        let now = Utc::now();
        let trial_days = request.trial_days.or(plan.trial_days).unwrap_or(0);

        let (status, period_start, period_end) = if trial_days > 0 {
            (
                SubscriptionStatus::Trialing,
                now,
                now + Duration::days(trial_days as i64),
            )
        } else {
            (
                SubscriptionStatus::Active,
                now,
                now + Duration::days(30),
            )
        };

        let subscription = Subscription {
            id: Uuid::new_v4(),
            organization_id: request.organization_id,
            plan_id: request.plan_id.clone(),
            status,
            current_period_start: period_start,
            current_period_end: period_end,
            stripe_subscription_id: None,
            stripe_customer_id: None,
            created_at: now,
            updated_at: now,
        };

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(subscription.id, subscription.clone());

        drop(subscriptions);

        let event_type = if trial_days > 0 {
            LifecycleEventType::TrialStarted
        } else {
            LifecycleEventType::Created
        };

        self.record_event(
            subscription.id,
            request.organization_id,
            event_type,
            None,
            Some(request.plan_id),
            request.metadata.unwrap_or_default(),
        )
        .await;

        Ok(subscription)
    }

    pub async fn get_subscription(&self, subscription_id: Uuid) -> Option<Subscription> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.get(&subscription_id).cloned()
    }

    pub async fn get_subscription_by_org(&self, organization_id: Uuid) -> Option<Subscription> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions
            .values()
            .find(|s| s.organization_id == organization_id && s.status != SubscriptionStatus::Canceled)
            .cloned()
    }

    pub async fn upgrade_subscription(
        &self,
        request: UpgradeDowngradeRequest,
    ) -> Result<SubscriptionChange, LifecycleError> {
        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions
            .get_mut(&request.subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        let plans = self.plans.read().await;
        let current_plan = plans
            .get(&subscription.plan_id)
            .ok_or_else(|| LifecycleError::PlanNotFound(subscription.plan_id.clone()))?;
        let new_plan = plans
            .get(&request.new_plan_id)
            .ok_or_else(|| LifecycleError::PlanNotFound(request.new_plan_id.clone()))?;

        let is_upgrade = self.is_upgrade(current_plan, new_plan);
        if !is_upgrade {
            return Err(LifecycleError::InvalidOperation(
                "Use downgrade for moving to a lower plan".to_string(),
            ));
        }

        let proration_amount = if request.prorate {
            self.calculate_proration(subscription, current_plan, new_plan)
        } else {
            None
        };

        let change = SubscriptionChange {
            subscription_id: subscription.id,
            change_type: ChangeType::Upgrade,
            effective_date: if request.immediate {
                Utc::now()
            } else {
                subscription.current_period_end
            },
            from_plan: subscription.plan_id.clone(),
            to_plan: request.new_plan_id.clone(),
            proration_amount,
            status: if request.immediate {
                ChangeStatus::Applied
            } else {
                ChangeStatus::Scheduled
            },
        };

        if request.immediate {
            let old_plan = subscription.plan_id.clone();
            let org_id = subscription.organization_id;
            subscription.plan_id = request.new_plan_id.clone();
            subscription.updated_at = Utc::now();

            drop(subscriptions);
            drop(plans);

            self.record_event(
                change.subscription_id,
                org_id,
                LifecycleEventType::Upgraded,
                Some(old_plan),
                Some(request.new_plan_id),
                HashMap::new(),
            )
            .await;
        } else {
            drop(subscriptions);
            drop(plans);

            let mut pending = self.pending_changes.write().await;
            pending.insert(change.subscription_id, change.clone());
        }

        Ok(change)
    }

    pub async fn downgrade_subscription(
        &self,
        request: UpgradeDowngradeRequest,
    ) -> Result<SubscriptionChange, LifecycleError> {
        let subscriptions = self.subscriptions.read().await;
        let subscription = subscriptions
            .get(&request.subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        let plans = self.plans.read().await;
        let current_plan = plans
            .get(&subscription.plan_id)
            .ok_or_else(|| LifecycleError::PlanNotFound(subscription.plan_id.clone()))?;
        let new_plan = plans
            .get(&request.new_plan_id)
            .ok_or_else(|| LifecycleError::PlanNotFound(request.new_plan_id.clone()))?;

        let is_upgrade = self.is_upgrade(current_plan, new_plan);
        if is_upgrade {
            return Err(LifecycleError::InvalidOperation(
                "Use upgrade for moving to a higher plan".to_string(),
            ));
        }

        let change = SubscriptionChange {
            subscription_id: subscription.id,
            change_type: ChangeType::Downgrade,
            effective_date: subscription.current_period_end,
            from_plan: subscription.plan_id.clone(),
            to_plan: request.new_plan_id.clone(),
            proration_amount: None,
            status: ChangeStatus::Scheduled,
        };

        let org_id = subscription.organization_id;
        drop(subscriptions);
        drop(plans);

        let mut pending = self.pending_changes.write().await;
        pending.insert(change.subscription_id, change.clone());

        self.record_event(
            change.subscription_id,
            org_id,
            LifecycleEventType::Downgraded,
            Some(change.from_plan.clone()),
            Some(change.to_plan.clone()),
            HashMap::from([("scheduled".to_string(), "true".to_string())]),
        )
        .await;

        Ok(change)
    }

    pub async fn cancel_subscription(
        &self,
        request: CancelSubscriptionRequest,
    ) -> Result<CancellationResult, LifecycleError> {
        if request.offer_retention {
            let offer = self.create_retention_offer(request.subscription_id).await?;
            return Ok(CancellationResult::RetentionOffered(offer));
        }

        let mut subscriptions = self.subscriptions.write().await;
        let subscription = subscriptions
            .get_mut(&request.subscription_id)
            .ok_or(LifecycleError::SubscriptionNotFound)?;

        let org_id = subscription.organization_id;
        let plan_id = subscription.plan_id.clone();

        if request.cancel_immediately {
            subscription.status = SubscriptionStatus::Canceled;
            subscription.updated_at = Utc::now();

            drop(subscriptions);

            self.record_event(
                request.subscription_id,
                org_id,
                LifecycleEventType::Cancelled,
                Some(plan_id),
                None,
                HashMap::from([
                    ("immediate".to_string(), "true".to_string()),
                    ("reason".to_string(), request.reason.unwrap_or_default()),
                ]),
            )
            .await;

            Ok(CancellationResult::CancelledImmediately)
        } else {
            drop(subscriptions);

            self.record_event(
                request.subscription_id,
                org_id,
                LifecycleEventType::CancellationRequested,
                None,
                None,
                HashMap::from([
                    ("reason".to_string(), request.reason.unwrap_or_default()),
                    ("feedback".to_string(), request.feedback.unwrap_or_default()),
                ]),
            )
            .await;

            let subscriptions = self.subscriptions.read().await;
            let subscription = subscriptions.get(&request.subscription_id);
            let end_date = subscription.map(|s| s.current_period_end).unwrap_or_else(Utc::now);

            Ok(CancellationResult::ScheduledForEndOfPeriod { end_date })
        }
    }
}
