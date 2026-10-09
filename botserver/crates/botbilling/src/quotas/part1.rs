use super::*;

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            usage_cache: Arc::new(RwLock::new(HashMap::new())),
            alert_thresholds: vec![80.0, 90.0, 100.0],
        }
    }

    pub fn with_thresholds(thresholds: Vec<f64>) -> Self {
        Self {
            usage_cache: Arc::new(RwLock::new(HashMap::new())),
            alert_thresholds: thresholds,
        }
    }

    pub async fn set_quotas(&self, quotas: OrganizationQuotas) {
        let mut cache = self.usage_cache.write().await;
        cache.insert(quotas.organization_id, quotas);
    }

    pub async fn get_quotas(&self, organization_id: Uuid) -> Option<OrganizationQuotas> {
        let cache = self.usage_cache.read().await;
        cache.get(&organization_id).cloned()
    }

    pub async fn check_quota(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
        increment: u64,
    ) -> Result<QuotaCheckResult, BillingError> {
        let cache = self.usage_cache.read().await;
        let quotas = cache
            .get(&organization_id)
            .ok_or(BillingError::SubscriptionNotFound)?;

        let (current, limit) = self.get_metric_values(quotas, metric);
        let new_value = current + increment;

        let (allowed, remaining, percentage) = match limit {
            LimitValue::Unlimited => (true, None, 0.0),
            LimitValue::Limited(max) => {
                let allowed = new_value <= max;
                let remaining = max.saturating_sub(new_value);
                let percentage = (new_value as f64 / max as f64) * 100.0;
                (allowed, Some(remaining), percentage)
            }
        };

        let alerts = self.generate_alerts(metric, percentage);

        Ok(QuotaCheckResult {
            allowed,
            metric,
            current: new_value,
            limit: limit.value(),
            remaining,
            percentage_used: percentage,
            alerts,
        })
    }

    pub async fn increment_usage(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
        amount: u64,
    ) -> Result<QuotaCheckResult, BillingError> {
        let check_result = self.check_quota(organization_id, metric, amount).await?;

        if check_result.allowed {
            let mut cache = self.usage_cache.write().await;
            if let Some(quotas) = cache.get_mut(&organization_id) {
                self.apply_increment(&mut quotas.usage, metric, amount);
            }
        }

        Ok(check_result)
    }

    pub async fn decrement_usage(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
        amount: u64,
    ) -> Result<(), BillingError> {
        let mut cache = self.usage_cache.write().await;
        let quotas = cache
            .get_mut(&organization_id)
            .ok_or(BillingError::SubscriptionNotFound)?;

        self.apply_decrement(&mut quotas.usage, metric, amount);
        Ok(())
    }

    pub async fn set_usage(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
        value: u64,
    ) -> Result<(), BillingError> {
        let mut cache = self.usage_cache.write().await;
        let quotas = cache
            .get_mut(&organization_id)
            .ok_or(BillingError::SubscriptionNotFound)?;

        self.set_metric_value(&mut quotas.usage, metric, value);
        Ok(())
    }

    pub async fn reset_daily_quotas(&self, organization_id: Uuid) -> Result<(), BillingError> {
        let mut cache = self.usage_cache.write().await;
        let quotas = cache
            .get_mut(&organization_id)
            .ok_or(BillingError::SubscriptionNotFound)?;

        quotas.usage.messages_today = 0;
        quotas.usage.api_calls_today = 0;
        quotas.usage.last_reset = Some(chrono::Utc::now());

        Ok(())
    }

    pub async fn reset_all_daily_quotas(&self) {
        let mut cache = self.usage_cache.write().await;
        let now = chrono::Utc::now();

        for quotas in cache.values_mut() {
            quotas.usage.messages_today = 0;
            quotas.usage.api_calls_today = 0;
            quotas.usage.last_reset = Some(now);
        }
    }

    pub async fn get_usage_summary(&self, organization_id: Uuid) -> Result<UsageSummary, BillingError> {
        let cache = self.usage_cache.read().await;
        let quotas = cache
            .get(&organization_id)
            .ok_or(BillingError::SubscriptionNotFound)?;

        let metrics = vec![
            UsageMetric::Messages,
            UsageMetric::StorageBytes,
            UsageMetric::ApiCalls,
            UsageMetric::Bots,
            UsageMetric::Users,
            UsageMetric::KbDocuments,
            UsageMetric::Apps,
            UsageMetric::EmailsPerDay,
            UsageMetric::BuildsPerDay,
            UsageMetric::UploadsPerHour,
        ];

        let items: Vec<UsageSummaryItem> = metrics
            .into_iter()
            .map(|metric| {
                let (current, limit) = self.get_metric_values(quotas, metric);
                let (remaining, percentage) = match limit {
                    LimitValue::Unlimited => (None, 0.0),
                    LimitValue::Limited(max) => {
                        let remaining = max.saturating_sub(current);
                        let percentage = (current as f64 / max as f64) * 100.0;
                        (Some(remaining), percentage)
                    }
                };

                UsageSummaryItem {
                    metric,
                    current,
                    limit: limit.value(),
                    remaining,
                    percentage_used: percentage,
                    is_unlimited: limit.is_unlimited(),
                }
            })
            .collect();

        Ok(UsageSummary {
            organization_id,
            plan_id: quotas.plan_id.clone(),
            items,
            period_start: quotas.period_start,
            period_end: quotas.period_end,
        })
    }

    pub async fn check_action(&self, organization_id: Uuid, metric: UsageMetric) -> QuotaAction {
        match self.check_quota(organization_id, metric, 1).await {
            Ok(result) => {
                if !result.allowed {
                    QuotaAction::Block {
                        message: format!(
                            "Quota exceeded for {:?}. Current: {}, Limit: {:?}",
                            metric, result.current, result.limit
                        ),
                    }
                } else if result.percentage_used >= 90.0 {
                    QuotaAction::Warn {
                        message: format!(
                            "Approaching quota limit for {:?}. {}% used.",
                            metric, result.percentage_used as u32
                        ),
                        percentage: result.percentage_used,
                    }
                } else {
                    QuotaAction::Allow
                }
            }
            Err(_) => QuotaAction::Block {
                message: "Unable to verify quota".to_string(),
            },
        }
    }

    pub(super) fn get_metric_values(&self, quotas: &OrganizationQuotas, metric: UsageMetric) -> (u64, LimitValue) {
        match metric {
            UsageMetric::Messages => (quotas.usage.messages_today, quotas.limits.messages_per_day),
            UsageMetric::StorageBytes => (quotas.usage.storage_bytes, quotas.limits.storage_bytes),
            UsageMetric::ApiCalls => (quotas.usage.api_calls_today, quotas.limits.api_calls_per_day),
            UsageMetric::Bots => (quotas.usage.bots, quotas.limits.bots),
            UsageMetric::Users => (quotas.usage.users, quotas.limits.users),
            UsageMetric::KbDocuments => (quotas.usage.kb_documents, quotas.limits.kb_documents),
            UsageMetric::Apps => (quotas.usage.apps, quotas.limits.apps),
            UsageMetric::EmailsPerDay => (quotas.usage.emails_today, quotas.limits.emails_per_day),
            UsageMetric::BuildsPerDay => (quotas.usage.builds_today, quotas.limits.builds_per_day),
            UsageMetric::UploadsPerHour => (quotas.usage.uploads_this_hour, quotas.limits.uploads_per_hour),
        }
    }

    pub(super) fn apply_increment(&self, usage: &mut QuotaUsage, metric: UsageMetric, amount: u64) {
        match metric {
            UsageMetric::Messages => usage.messages_today += amount,
            UsageMetric::StorageBytes => usage.storage_bytes += amount,
            UsageMetric::ApiCalls => usage.api_calls_today += amount,
            UsageMetric::Bots => usage.bots += amount,
            UsageMetric::Users => usage.users += amount,
            UsageMetric::KbDocuments => usage.kb_documents += amount,
            UsageMetric::Apps => usage.apps += amount,
            UsageMetric::EmailsPerDay => usage.emails_today += amount,
            UsageMetric::BuildsPerDay => usage.builds_today += amount,
            UsageMetric::UploadsPerHour => usage.uploads_this_hour += amount,
        }
    }

    pub(super) fn apply_decrement(&self, usage: &mut QuotaUsage, metric: UsageMetric, amount: u64) {
        match metric {
            UsageMetric::Messages => usage.messages_today = usage.messages_today.saturating_sub(amount),
            UsageMetric::StorageBytes => usage.storage_bytes = usage.storage_bytes.saturating_sub(amount),
            UsageMetric::ApiCalls => usage.api_calls_today = usage.api_calls_today.saturating_sub(amount),
            UsageMetric::Bots => usage.bots = usage.bots.saturating_sub(amount),
            UsageMetric::Users => usage.users = usage.users.saturating_sub(amount),
            UsageMetric::KbDocuments => usage.kb_documents = usage.kb_documents.saturating_sub(amount),
            UsageMetric::Apps => usage.apps = usage.apps.saturating_sub(amount),
            UsageMetric::EmailsPerDay => usage.emails_today = usage.emails_today.saturating_sub(amount),
            UsageMetric::BuildsPerDay => usage.builds_today = usage.builds_today.saturating_sub(amount),
            UsageMetric::UploadsPerHour => usage.uploads_this_hour = usage.uploads_this_hour.saturating_sub(amount),
        }
    }

    pub(super) fn set_metric_value(&self, usage: &mut QuotaUsage, metric: UsageMetric, value: u64) {
        match metric {
            UsageMetric::Messages => usage.messages_today = value,
            UsageMetric::StorageBytes => usage.storage_bytes = value,
            UsageMetric::ApiCalls => usage.api_calls_today = value,
            UsageMetric::Bots => usage.bots = value,
            UsageMetric::Users => usage.users = value,
            UsageMetric::KbDocuments => usage.kb_documents = value,
            UsageMetric::Apps => usage.apps = value,
            UsageMetric::EmailsPerDay => usage.emails_today = value,
            UsageMetric::BuildsPerDay => usage.builds_today = value,
            UsageMetric::UploadsPerHour => usage.uploads_this_hour = value,
        }
    }

    pub(super) fn generate_alerts(&self, metric: UsageMetric, percentage: f64) -> Vec<QuotaAlert> {
        self.alert_thresholds
            .iter()
            .filter(|&&threshold| percentage >= threshold)
            .map(|&threshold| QuotaAlert {
                metric,
                threshold,
                current_percentage: percentage,
                message: self.alert_message(metric, threshold, percentage),
            })
            .collect()
    }
}
