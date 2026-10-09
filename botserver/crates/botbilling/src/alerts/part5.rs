use super::*;

impl AlertManager {
    pub fn new() -> Self {
        Self {
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(HashMap::new())),
            notification_prefs: Arc::new(RwLock::new(HashMap::new())),
            thresholds: AlertThresholds::default(),
            cooldown_seconds: 3600,
            max_history_per_org: 100,
            notification_handlers: Arc::new(RwLock::new(Vec::new())),
            pool: None,
        }
    }

    /// Attach a database pool so alerts and history persist across restarts
    /// (#829: alert history must be durable, not an in-memory map).
    pub fn with_pool(mut self, pool: DbPool) -> Self {
        self.pool = Some(pool);
        self
    }

    pub fn with_thresholds(mut self, thresholds: AlertThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    pub fn with_cooldown(mut self, seconds: u64) -> Self {
        self.cooldown_seconds = seconds;
        self
    }

    pub async fn register_handler(&self, handler: Arc<dyn NotificationHandler>) {
        let mut handlers = self.notification_handlers.write().await;
        handlers.push(handler);
    }

    pub async fn set_notification_preferences(
        &self,
        org_id: Uuid,
        prefs: NotificationPreferences,
    ) {
        let mut all_prefs = self.notification_prefs.write().await;
        all_prefs.insert(org_id, prefs);
    }

    pub async fn get_notification_preferences(
        &self,
        org_id: Uuid,
    ) -> NotificationPreferences {
        let prefs = self.notification_prefs.read().await;
        prefs.get(&org_id).cloned().unwrap_or_default()
    }

    pub async fn check_and_alert(
        &self,
        org_id: Uuid,
        metric: UsageMetric,
        current_usage: u64,
        limit: u64,
    ) -> Option<UsageAlert> {
        if limit == 0 {
            return None;
        }

        let percentage = (current_usage as f64 / limit as f64) * 100.0;
        let severity = self.thresholds.get_severity(percentage)?;
        let threshold = match severity {
            AlertSeverity::Warning => self.thresholds.warning,
            AlertSeverity::Critical => self.thresholds.critical,
            AlertSeverity::Exceeded => self.thresholds.exceeded,
        };

        if self.is_in_cooldown(org_id, metric, severity.clone()).await {
            return None;
        }

        let alert = UsageAlert::new(
            org_id,
            metric,
            severity,
            current_usage,
            limit,
            percentage,
            threshold,
        );

        self.store_alert(org_id, alert.clone()).await;
        self.send_notifications(org_id, &alert).await;

        Some(alert)
    }

    pub async fn check_all_metrics(
        &self,
        org_id: Uuid,
        usage: &UsageSnapshot,
    ) -> Vec<UsageAlert> {
        let mut alerts = Vec::new();

        for (metric, current, limit) in usage.iter_metrics() {
            if let Some(alert) = self.check_and_alert(org_id, metric, current, limit).await {
                alerts.push(alert);
            }
        }

        alerts
    }

    pub async fn get_active_alerts(&self, org_id: Uuid) -> Vec<UsageAlert> {
        let alerts = self.active_alerts.read().await;
        alerts.get(&org_id).cloned().unwrap_or_default()
    }

    pub async fn get_alert_history(
        &self,
        org_id: Uuid,
        limit: Option<usize>,
    ) -> Vec<UsageAlert> {
        if self.pool.is_some() {
            let db_alerts = self.load_history_from_db(org_id, limit.unwrap_or(self.max_history_per_org)).await;
            if !db_alerts.is_empty() {
                return db_alerts;
            }
        }
        let history = self.alert_history.read().await;
        let mut alerts = history.get(&org_id).cloned().unwrap_or_default();

        if let Some(limit) = limit {
            alerts.truncate(limit);
        }

        alerts
    }

    pub async fn acknowledge_alert(
        &self,
        org_id: Uuid,
        alert_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AlertError> {
        let mut alerts = self.active_alerts.write().await;
        let org_alerts = alerts.get_mut(&org_id).ok_or(AlertError::NotFound)?;

        let alert = org_alerts
            .iter_mut()
            .find(|a| a.id == alert_id)
            .ok_or(AlertError::NotFound)?;

        alert.acknowledge(user_id);

        Ok(())
    }

    pub async fn dismiss_alert(
        &self,
        org_id: Uuid,
        alert_id: Uuid,
    ) -> Result<UsageAlert, AlertError> {
        let mut alerts = self.active_alerts.write().await;
        let org_alerts = alerts.get_mut(&org_id).ok_or(AlertError::NotFound)?;

        let index = org_alerts
            .iter()
            .position(|a| a.id == alert_id)
            .ok_or(AlertError::NotFound)?;

        let alert = org_alerts.remove(index);

        self.add_to_history(org_id, alert.clone()).await;

        Ok(alert)
    }

    pub async fn clear_alerts(&self, org_id: Uuid) {
        let mut alerts = self.active_alerts.write().await;
        if let Some(org_alerts) = alerts.remove(&org_id) {
            for alert in org_alerts {
                self.add_to_history(org_id, alert).await;
            }
        }
    }

    pub async fn get_alert_counts(&self, org_id: Uuid) -> AlertCounts {
        let alerts = self.active_alerts.read().await;
        let org_alerts = alerts.get(&org_id);

        let mut counts = AlertCounts::default();

        if let Some(alerts) = org_alerts {
            for alert in alerts {
                match alert.severity {
                    AlertSeverity::Warning => counts.warning += 1,
                    AlertSeverity::Critical => counts.critical += 1,
                    AlertSeverity::Exceeded => counts.exceeded += 1,
                }
                counts.total += 1;
            }
        }

        counts
    }

    async fn is_in_cooldown(
        &self,
        org_id: Uuid,
        metric: UsageMetric,
        severity: AlertSeverity,
    ) -> bool {
        let alerts = self.active_alerts.read().await;
        let org_alerts = match alerts.get(&org_id) {
            Some(a) => a,
            None => return false,
        };

        let cooldown_threshold = Utc::now()
            - chrono::Duration::seconds(self.cooldown_seconds as i64);

        org_alerts.iter().any(|alert| {
            alert.metric == metric
                && alert.severity == severity
                && alert.created_at > cooldown_threshold
        })
    }

    async fn store_alert(&self, org_id: Uuid, alert: UsageAlert) {
        let mut alerts = self.active_alerts.write().await;
        let org_alerts = alerts.entry(org_id).or_insert_with(Vec::new);

        org_alerts.retain(|a| {
            a.metric != alert.metric || a.severity.priority() >= alert.severity.priority()
        });

        org_alerts.push(alert.clone());
        drop(alerts);

        self.persist_alert(&alert).await;
    }

    async fn add_to_history(&self, org_id: Uuid, alert: UsageAlert) {
        let mut history = self.alert_history.write().await;
        let org_history = history.entry(org_id).or_insert_with(Vec::new);

        org_history.insert(0, alert.clone());

        if org_history.len() > self.max_history_per_org {
            org_history.truncate(self.max_history_per_org);
        }
        drop(history);

        self.persist_history(&alert).await;
    }

    /// INSERT the alert into `billing_usage_alerts` (active state) when a pool
    /// is attached — durable across restarts.
    async fn persist_alert(&self, alert: &UsageAlert) {
        let Some(pool) = self.pool.clone() else { return };
        let alert = alert.clone();
        let metric = serde_json::to_string(&alert.metric).unwrap_or_else(|_| "unknown".into());
        let channels = serde_json::to_string(&alert.notification_channels)
            .unwrap_or_else(|_| "[]".into());
        let now = Utc::now();
        let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
            sql_query(
                "INSERT INTO billing_usage_alerts \
                 (id, org_id, bot_id, metric, severity, current_usage, usage_limit, percentage, \
                  threshold, message, notification_sent, notification_channels, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::jsonb, $13, $13) \
                 ON CONFLICT (id) DO UPDATE SET \
                   current_usage = EXCLUDED.current_usage, \
                   percentage = EXCLUDED.percentage, \
                   message = EXCLUDED.message, \
                   updated_at = EXCLUDED.updated_at",
            )
            .bind::<DieselUuid, _>(alert.id)
            .bind::<DieselUuid, _>(alert.organization_id)
            .bind::<DieselUuid, _>(Uuid::nil())
            .bind::<Text, _>(&metric)
            .bind::<Text, _>(alert.severity.as_str())
            .bind::<BigInt, _>(alert.current_usage as i64)
            .bind::<BigInt, _>(alert.limit as i64)
            .bind::<Text, _>(format!("{:.2}", alert.percentage))
            .bind::<Text, _>(format!("{:.2}", alert.threshold))
            .bind::<Text, _>(&alert.message)
            .bind::<Bool, _>(alert.notification_sent)
            .bind::<Text, _>(&channels)
            .bind::<Timestamptz, _>(now)
            .execute(&mut conn)
            .map_err(|e| format!("persist alert: {e}"))?;
            Ok(())
        })
        .await;
        if let Err(e) = result {
            tracing::warn!("Failed to persist usage alert for org {}: {}", alert.organization_id, e);
        }
    }
}
