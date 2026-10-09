use super::*;

impl UsageMeteringService {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(MeteringStorage::default())),
            aggregation_interval_secs: 3600,
        }
    }

    pub fn with_aggregation_interval(interval_secs: u64) -> Self {
        Self {
            storage: Arc::new(RwLock::new(MeteringStorage::default())),
            aggregation_interval_secs: interval_secs,
        }
    }

    pub async fn record_event(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
        value: i64,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<UsageEvent, BillingError> {
        let event = UsageEvent {
            id: Uuid::new_v4(),
            organization_id,
            metric,
            value,
            timestamp: Utc::now(),
            metadata,
        };

        let mut storage = self.storage.write().await;
        storage
            .events
            .entry(organization_id)
            .or_default()
            .push(event.clone());

        self.update_aggregation(&mut storage, organization_id, metric, value);

        Ok(event)
    }

    pub async fn record_increment(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
        amount: u64,
    ) -> Result<UsageEvent, BillingError> {
        self.record_event(organization_id, metric, amount as i64, None).await
    }

    pub async fn record_decrement(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
        amount: u64,
    ) -> Result<UsageEvent, BillingError> {
        self.record_event(organization_id, metric, -(amount as i64), None).await
    }

    pub async fn get_current_usage(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
    ) -> Result<u64, BillingError> {
        let storage = self.storage.read().await;
        let aggregated = storage
            .aggregated
            .get(&organization_id)
            .and_then(|m| m.get(&metric));

        Ok(aggregated.map(|a| a.total).unwrap_or(0))
    }

    pub async fn get_aggregated_usage(
        &self,
        organization_id: Uuid,
        metric: UsageMetric,
    ) -> Result<Option<AggregatedUsage>, BillingError> {
        let storage = self.storage.read().await;
        let aggregated = storage
            .aggregated
            .get(&organization_id)
            .and_then(|m| m.get(&metric))
            .cloned();

        Ok(aggregated)
    }

    pub async fn get_all_metrics(
        &self,
        organization_id: Uuid,
    ) -> Result<HashMap<UsageMetric, AggregatedUsage>, BillingError> {
        let storage = self.storage.read().await;
        let metrics = storage
            .aggregated
            .get(&organization_id)
            .cloned()
            .unwrap_or_default();

        Ok(metrics)
    }

    pub async fn generate_report(
        &self,
        organization_id: Uuid,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<MeteringReport, BillingError> {
        let storage = self.storage.read().await;
        let aggregated = storage.aggregated.get(&organization_id);

        let metrics = [
            UsageMetric::Messages,
            UsageMetric::StorageBytes,
            UsageMetric::ApiCalls,
            UsageMetric::Bots,
            UsageMetric::Users,
            UsageMetric::KbDocuments,
            UsageMetric::Apps,
        ];

        let metric_reports: Vec<MetricReport> = metrics
            .iter()
            .map(|&metric| {
                let agg = aggregated.and_then(|m| m.get(&metric));
                MetricReport {
                    metric,
                    current_value: agg.map(|a| a.total).unwrap_or(0),
                    period_total: agg.map(|a| a.total).unwrap_or(0),
                    period_average: agg.map(|a| a.average).unwrap_or(0.0),
                    peak_value: agg.and_then(|a| a.max).unwrap_or(0),
                    trend: self.calculate_trend(&storage, organization_id, metric),
                }
            })
            .collect();

        Ok(MeteringReport {
            organization_id,
            period_start,
            period_end,
            metrics: metric_reports,
            generated_at: Utc::now(),
        })
    }

    pub async fn create_daily_snapshot(
        &self,
        organization_id: Uuid,
    ) -> Result<DailySnapshot, BillingError> {
        let mut storage = self.storage.write().await;
        let metrics = storage
            .aggregated
            .get(&organization_id)
            .map(|m| m.iter().map(|(&k, v)| (k, v.total)).collect())
            .unwrap_or_default();

        let snapshot = DailySnapshot {
            organization_id,
            date: Utc::now().date_naive(),
            metrics,
            created_at: Utc::now(),
        };

        storage
            .daily_snapshots
            .entry(organization_id)
            .or_default()
            .push(snapshot.clone());

        Ok(snapshot)
    }

    pub async fn get_daily_snapshots(
        &self,
        organization_id: Uuid,
        days: usize,
    ) -> Result<Vec<DailySnapshot>, BillingError> {
        let storage = self.storage.read().await;
        let snapshots = storage
            .daily_snapshots
            .get(&organization_id)
            .map(|s| {
                let len = s.len();
                let start = len.saturating_sub(days);
                s[start..].to_vec()
            })
            .unwrap_or_default();

        Ok(snapshots)
    }

    pub async fn reset_daily_metrics(&self, organization_id: Uuid) -> Result<(), BillingError> {
        let mut storage = self.storage.write().await;

        if let Some(org_aggregated) = storage.aggregated.get_mut(&organization_id) {
            let now = Utc::now();
            for metric in [UsageMetric::Messages, UsageMetric::ApiCalls] {
                if let Some(agg) = org_aggregated.get_mut(&metric) {
                    agg.total = 0;
                    agg.count = 0;
                    agg.min = None;
                    agg.max = None;
                    agg.average = 0.0;
                    agg.period_start = now;
                    agg.last_updated = now;
                }
            }
        }

        Ok(())
    }

    pub async fn prune_old_events(&self, retention_days: i64) -> Result<usize, BillingError> {
        let mut storage = self.storage.write().await;
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);
        let mut pruned = 0;

        for events in storage.events.values_mut() {
            let original_len = events.len();
            events.retain(|e| e.timestamp > cutoff);
            pruned += original_len - events.len();
        }

        for snapshots in storage.daily_snapshots.values_mut() {
            let original_len = snapshots.len();
            snapshots.retain(|s| s.created_at > cutoff);
            pruned += original_len - snapshots.len();
        }

        Ok(pruned)
    }

    pub(super) fn update_aggregation(
        &self,
        storage: &mut MeteringStorage,
        organization_id: Uuid,
        metric: UsageMetric,
        value: i64,
    ) {
        let now = Utc::now();
        let org_aggregated = storage.aggregated.entry(organization_id).or_default();
        let agg = org_aggregated.entry(metric).or_insert_with(|| AggregatedUsage {
            metric,
            total: 0,
            count: 0,
            min: None,
            max: None,
            average: 0.0,
            period_start: now,
            period_end: now,
            last_updated: now,
        });

        if value >= 0 {
            agg.total = agg.total.saturating_add(value as u64);
        } else {
            agg.total = agg.total.saturating_sub(value.unsigned_abs());
        }

        agg.count += 1;
        agg.average = agg.total as f64 / agg.count as f64;

        let abs_value = value.unsigned_abs();
        agg.min = Some(agg.min.map(|m| m.min(abs_value)).unwrap_or(abs_value));
        agg.max = Some(agg.max.map(|m| m.max(abs_value)).unwrap_or(abs_value));
        agg.period_end = now;
        agg.last_updated = now;
    }
}
