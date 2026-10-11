use super::*;

impl AlertManager {
    pub(crate) async fn persist_history(&self, alert: &UsageAlert) {
        let Some(pool) = self.pool.clone() else { return };
        let alert = alert.clone();
        let metric = serde_json::to_string(&alert.metric).unwrap_or_else(|_| "unknown".into());
        let now = Utc::now();
        let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
            sql_query(
                "INSERT INTO billing_alert_history \
                 (id, org_id, bot_id, alert_id, metric, severity, current_usage, usage_limit, \
                  percentage, message, acknowledged_at, acknowledged_by, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            )
            .bind::<DieselUuid, _>(Uuid::new_v4())
            .bind::<DieselUuid, _>(alert.organization_id)
            .bind::<DieselUuid, _>(Uuid::nil())
            .bind::<DieselUuid, _>(alert.id)
            .bind::<Text, _>(&metric)
            .bind::<Text, _>(alert.severity.as_str())
            .bind::<BigInt, _>(alert.current_usage as i64)
            .bind::<BigInt, _>(alert.limit as i64)
            .bind::<Text, _>(format!("{:.2}", alert.percentage))
            .bind::<Text, _>(&alert.message)
            .bind::<Nullable<Timestamptz>, _>(alert.acknowledged_at)
            .bind::<Nullable<DieselUuid>, _>(alert.acknowledged_by)
            .bind::<Timestamptz, _>(now)
            .execute(&mut conn)
            .map_err(|e| format!("persist alert history: {e}"))?;
            Ok(())
        })
        .await;
        if let Err(e) = result {
            tracing::warn!("Failed to persist alert history for org {}: {}", alert.organization_id, e);
        }
    }

    /// Load recent history from the DB (falling back to memory when no pool).
    pub(crate) async fn load_history_from_db(&self, org_id: Uuid, limit: usize) -> Vec<UsageAlert> {
        let Some(pool) = self.pool.clone() else { return Vec::new() };
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<UsageAlert>, String> {
            let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
            #[derive(diesel::QueryableByName)]
            struct HistoryRow {
                #[diesel(sql_type = DieselUuid)]
                alert_id: Uuid,
                #[diesel(sql_type = Text)]
                metric: String,
                #[diesel(sql_type = Text)]
                severity: String,
                #[diesel(sql_type = BigInt)]
                current_usage: i64,
                #[diesel(sql_type = BigInt)]
                usage_limit: i64,
                #[diesel(sql_type = Text)]
                percentage: String,
                #[diesel(sql_type = Text)]
                message: String,
                #[diesel(sql_type = Nullable<Timestamptz>)]
                acknowledged_at: Option<DateTime<Utc>>,
                #[diesel(sql_type = Nullable<DieselUuid>)]
                acknowledged_by: Option<Uuid>,
                #[diesel(sql_type = Timestamptz)]
                created_at: DateTime<Utc>,
            }
            let rows = sql_query(
                "SELECT alert_id, metric, severity, current_usage, usage_limit, percentage, \
                        message, acknowledged_at, acknowledged_by, created_at \
                 FROM billing_alert_history WHERE org_id = $1 ORDER BY created_at DESC LIMIT $2",
            )
            .bind::<DieselUuid, _>(org_id)
            .bind::<BigInt, _>(limit as i64)
            .load::<HistoryRow>(&mut conn)
            .map_err(|e| format!("load alert history: {e}"))?;
            Ok(rows
                .into_iter()
                .map(|r| UsageAlert {
                    id: r.alert_id,
                    organization_id: org_id,
                    metric: serde_json::from_str(&r.metric).unwrap_or(UsageMetric::Messages),
                    severity: match r.severity.as_str() {
                        "critical" => AlertSeverity::Critical,
                        "exceeded" => AlertSeverity::Exceeded,
                        _ => AlertSeverity::Warning,
                    },
                    current_usage: r.current_usage.max(0) as u64,
                    limit: r.usage_limit.max(0) as u64,
                    percentage: r.percentage.parse().unwrap_or(0.0),
                    threshold: 0.0,
                    message: r.message,
                    created_at: r.created_at,
                    acknowledged_at: r.acknowledged_at,
                    acknowledged_by: r.acknowledged_by,
                    notification_sent: false,
                    notification_channels: Vec::new(),
                })
                .collect())
        })
        .await;
        match result {
            Ok(Ok(alerts)) => alerts,
            Ok(Err(e)) => {
                tracing::warn!("Failed to load alert history for org {org_id}: {e}");
                Vec::new()
            }
            Err(e) => {
                tracing::warn!("Alert history load task panicked: {e}");
                Vec::new()
            }
        }
    }

    pub(crate) async fn send_notifications(&self, org_id: Uuid, alert: &UsageAlert) {
        let prefs = self.get_notification_preferences(org_id).await;

        if !prefs.enabled {
            return;
        }

        if !prefs.should_notify(alert.severity.clone()) {
            return;
        }

        let handlers = self.notification_handlers.read().await;
        let notification = AlertNotification::from_alert(alert, &prefs);

        for handler in handlers.iter() {
            if prefs.channels.contains(&handler.channel()) {
                if let Err(e) = handler.send(&notification).await {
                    tracing::warn!(
                        "Failed to send {} notification for org {}: {}",
                        handler.channel().as_str(),
                        org_id,
                        e
                    );
                }
            }
        }
    }
}
