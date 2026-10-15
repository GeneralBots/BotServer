use super::*;

impl MetricsCollector {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);

        let health = SystemHealth {
            status: HealthStatus::Unknown,
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            active_connections: 0,
            requests_per_second: 0.0,
            error_rate_percent: 0.0,
            average_latency_ms: 0.0,
            uptime_seconds: 0,
            last_check: Utc::now(),
            components: Vec::new(),
        };

        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_rules: Arc::new(RwLock::new(Vec::new())),
            health: Arc::new(RwLock::new(health)),
            broadcast_tx: tx,
            request_counter: AtomicU64::new(0),
            error_counter: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            start_time: Utc::now(),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MonitoringMessage> {
        self.broadcast_tx.subscribe()
    }

    pub async fn record_metric(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let now = Utc::now();
        let data_point = MetricDataPoint {
            timestamp: now,
            value,
            labels: labels.clone(),
        };

        let mut metrics = self.metrics.write().await;

        if let Some(metric) = metrics.get_mut(name) {
            metric.current_value = value;
            metric.updated_at = now;

            metric.data_points.push(data_point);
            if metric.data_points.len() > 1000 {
                metric.data_points.remove(0);
            }

            let _ = self.broadcast_tx.send(MonitoringMessage::MetricUpdate {
                metric: metric.clone(),
            });
        } else {
            let metric = Metric {
                name: name.to_string(),
                metric_type: MetricType::Gauge,
                description: String::new(),
                unit: None,
                labels,
                current_value: value,
                data_points: vec![data_point],
                updated_at: now,
            };

            let _ = self.broadcast_tx.send(MonitoringMessage::MetricUpdate {
                metric: metric.clone(),
            });

            metrics.insert(name.to_string(), metric);
        }

        drop(metrics);

        self.check_alert_rules(name, value).await;
    }

    pub async fn increment_counter(&self, name: &str, labels: HashMap<String, String>) {
        let metrics = self.metrics.read().await;
        let current = metrics.get(name).map(|m| m.current_value).unwrap_or(0.0);
        drop(metrics);

        self.record_metric(name, current + 1.0, labels).await;
    }

    pub async fn record_histogram(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        self.record_metric(name, value, labels).await;
    }

    pub async fn register_metric(
        &self,
        name: &str,
        metric_type: MetricType,
        description: &str,
        unit: Option<String>,
    ) {
        let metric = Metric {
            name: name.to_string(),
            metric_type,
            description: description.to_string(),
            unit,
            labels: HashMap::new(),
            current_value: 0.0,
            data_points: Vec::new(),
            updated_at: Utc::now(),
        };

        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), metric);
    }

    pub async fn add_alert_rule(&self, rule: AlertRule) {
        let mut rules = self.alert_rules.write().await;
        rules.push(rule);
    }

    pub async fn get_alert_rules(&self) -> Vec<AlertRule> {
        let rules = self.alert_rules.read().await;
        rules.clone()
    }

    pub async fn replace_alert_rule(&self, rule: AlertRule) {
        let mut rules = self.alert_rules.write().await;
        if let Some(existing) = rules.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule;
        } else {
            rules.push(rule);
        }
    }

    pub async fn remove_alert_rule(&self, rule_id: Uuid) -> bool {
        let mut rules = self.alert_rules.write().await;
        let initial_len = rules.len();
        rules.retain(|r| r.id != rule_id);
        rules.len() < initial_len
    }

    async fn check_alert_rules(&self, metric_name: &str, value: f64) {
        let rules = self.alert_rules.read().await;
        let relevant_rules: Vec<_> = rules
            .iter()
            .filter(|r| r.enabled && r.metric_name == metric_name)
            .cloned()
            .collect();
        drop(rules);

        for rule in relevant_rules {
            let should_fire = rule.condition.evaluate(value, None, 0);

            if should_fire {
                self.fire_alert(&rule, value).await;
            } else {
                self.resolve_alert_for_rule(rule.id).await;
            }
        }
    }

    async fn fire_alert(&self, rule: &AlertRule, value: f64) {
        let mut alerts = self.alerts.write().await;

        let existing = alerts.values().find(|a| {
            a.rule_id == rule.id && matches!(a.status, AlertStatus::Firing | AlertStatus::Acknowledged)
        });

        if existing.is_some() {
            return;
        }

        let threshold = match &rule.condition {
            AlertCondition::GreaterThan(t)
            | AlertCondition::LessThan(t)
            | AlertCondition::Equals(t)
            | AlertCondition::NotEquals(t)
            | AlertCondition::GreaterThanOrEqual(t)
            | AlertCondition::LessThanOrEqual(t) => *t,
            AlertCondition::AbsentFor(s) => *s as f64,
            AlertCondition::RateOfChange { threshold, .. } => *threshold,
        };

        let alert = Alert {
            id: Uuid::new_v4(),
            rule_id: rule.id,
            rule_name: rule.name.clone(),
            severity: rule.severity.clone(),
            status: AlertStatus::Firing,
            metric_name: rule.metric_name.clone(),
            metric_value: value,
            threshold,
            message: format!(
                "{}: {} is {} (threshold: {})",
                rule.name, rule.metric_name, value, threshold
            ),
            labels: rule.labels.clone(),
            annotations: rule.annotations.clone(),
            started_at: Utc::now(),
            resolved_at: None,
            acknowledged_at: None,
            acknowledged_by: None,
        };

        let _ = self.broadcast_tx.send(MonitoringMessage::AlertFired {
            alert: alert.clone(),
        });

        alerts.insert(alert.id, alert);
    }

    async fn resolve_alert_for_rule(&self, rule_id: Uuid) {
        let mut alerts = self.alerts.write().await;

        let alert_ids: Vec<_> = alerts
            .iter()
            .filter(|(_, a)| a.rule_id == rule_id && a.status == AlertStatus::Firing)
            .map(|(id, _)| *id)
            .collect();

        for alert_id in alert_ids {
            if let Some(alert) = alerts.get_mut(&alert_id) {
                alert.status = AlertStatus::Resolved;
                alert.resolved_at = Some(Utc::now());

                let _ = self
                    .broadcast_tx
                    .send(MonitoringMessage::AlertResolved { alert_id });
            }
        }
    }

    pub async fn acknowledge_alert(&self, alert_id: Uuid, acknowledged_by: &str) -> bool {
        let mut alerts = self.alerts.write().await;

        if let Some(alert) = alerts.get_mut(&alert_id) {
            if alert.status == AlertStatus::Firing {
                alert.status = AlertStatus::Acknowledged;
                alert.acknowledged_at = Some(Utc::now());
                alert.acknowledged_by = Some(acknowledged_by.to_string());
                return true;
            }
        }

        false
    }

    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts
            .values()
            .filter(|a| matches!(a.status, AlertStatus::Firing | AlertStatus::Acknowledged))
            .cloned()
            .collect()
    }

    pub async fn get_recent_alerts(&self, limit: usize) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        let mut all_alerts: Vec<_> = alerts.values().cloned().collect();
        all_alerts.sort_by_key(|a| std::cmp::Reverse(a.started_at));
        all_alerts.truncate(limit);
        all_alerts
    }

    pub fn record_request(&self, latency_ms: u64, is_error: bool) {
        self.request_counter.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);

        if is_error {
            self.error_counter.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
