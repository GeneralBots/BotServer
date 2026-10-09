use super::*;

impl Default for UsageMeteringService {
    fn default() -> Self {
        Self::new()
    }
}
pub async fn metering_aggregation_job(metering_service: Arc<UsageMeteringService>) {
    let interval = metering_service.aggregation_interval();
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(interval));

    loop {
        ticker.tick().await;

        if let Err(e) = metering_service.prune_old_events(30).await {
            tracing::warn!("Failed to prune old metering events: {e}");
        }
    }
}
pub async fn daily_snapshot_job(
    metering_service: Arc<UsageMeteringService>,
    organization_ids: Arc<RwLock<Vec<Uuid>>>,
) {
    loop {
        let now = Utc::now();
        let tomorrow = (now + chrono::Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));

        if let Some(next_snapshot) = tomorrow {
            let duration = next_snapshot - now;
            if let Ok(std_duration) = duration.to_std() {
                tokio::time::sleep(std_duration).await;
            }
        }

        let org_ids = organization_ids.read().await.clone();
        for org_id in org_ids {
            if let Err(e) = metering_service.create_daily_snapshot(org_id).await {
                tracing::warn!("Failed to create daily snapshot for org {org_id}: {e}");
            }
            if let Err(e) = metering_service.reset_daily_metrics(org_id).await {
                tracing::warn!("Failed to reset daily metrics for org {org_id}: {e}");
            }
        }

        tracing::info!("Daily metering snapshots completed");
    }
}
