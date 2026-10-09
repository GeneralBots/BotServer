use super::*;

impl QuotaMiddleware {
    pub fn new(quota_manager: Arc<QuotaManager>) -> Self {
        Self { quota_manager }
    }

    pub async fn check_and_increment(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
    ) -> Result<QuotaCheckResult, BillingError> {
        self.quota_manager.increment_usage(organization_id, metric, 1).await
    }

    pub async fn check_storage(
        &self,
        organization_id: Uuid,
        bytes: u64,
    ) -> Result<QuotaCheckResult, BillingError> {
        self.quota_manager.check_quota(organization_id, UsageMetric::StorageBytes, bytes).await
    }

    pub async fn add_storage(
        &self,
        organization_id: Uuid,
        bytes: u64,
    ) -> Result<QuotaCheckResult, BillingError> {
        self.quota_manager.increment_usage(organization_id, UsageMetric::StorageBytes, bytes).await
    }

    pub async fn remove_storage(
        &self,
        organization_id: Uuid,
        bytes: u64,
    ) -> Result<(), BillingError> {
        self.quota_manager.decrement_usage(organization_id, UsageMetric::StorageBytes, bytes).await
    }
}
pub async fn daily_quota_reset_job(quota_manager: Arc<QuotaManager>) {
    loop {
        let now = chrono::Utc::now();
        let tomorrow = (now + chrono::Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc));

        if let Some(next_reset) = tomorrow {
            let duration = next_reset - now;
            if let Ok(std_duration) = duration.to_std() {
                tokio::time::sleep(std_duration).await;
            }
        }

        quota_manager.reset_all_daily_quotas().await;
        tracing::info!("Daily quotas reset completed");
    }
}
