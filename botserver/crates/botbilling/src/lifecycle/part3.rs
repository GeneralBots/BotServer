use super::*;

impl SubscriptionLifecycleService {
    pub(crate) async fn record_event(
        &self,
        subscription_id: Uuid,
        organization_id: Uuid,
        event_type: LifecycleEventType,
        from_plan: Option<String>,
        to_plan: Option<String>,
        metadata: HashMap<String, String>,
    ) {
        let event = SubscriptionLifecycleEvent {
            id: Uuid::new_v4(),
            subscription_id,
            organization_id,
            event_type,
            from_plan,
            to_plan,
            timestamp: Utc::now(),
            metadata,
            processed: false,
        };

        let mut events = self.events.write().await;
        events.push(event);
    }

    pub(super) fn is_upgrade(&self, current: &PlanConfig, new: &PlanConfig) -> bool {
        let current_value = self.plan_value(current);
        let new_value = self.plan_value(new);
        new_value > current_value
    }

    pub(super) fn plan_value(&self, plan: &PlanConfig) -> u64 {
        match &plan.price {
            PlanPrice::Free => 0,
            PlanPrice::Fixed { amount, .. } => *amount,
            PlanPrice::Custom => u64::MAX,
        }
    }

    pub(super) fn calculate_proration(
        &self,
        subscription: &Subscription,
        _current_plan: &PlanConfig,
        _new_plan: &PlanConfig,
    ) -> Option<i64> {
        let now = Utc::now();
        let period_total = (subscription.current_period_end - subscription.current_period_start)
            .num_days() as f64;
        let days_remaining = (subscription.current_period_end - now).num_days() as f64;

        if period_total > 0.0 {
            let ratio = days_remaining / period_total;
            Some((ratio * 100.0) as i64)
        } else {
            None
        }
    }
}
