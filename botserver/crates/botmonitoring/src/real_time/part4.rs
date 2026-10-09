use super::*;

impl MetricsCollector {
    pub async fn start_background_collection(self: Arc<Self>) {
        let collector = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;
                collector.update_system_health().await;
            }
        });

        let collector = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            loop {
                interval.tick().await;
                collector.record_builtin_metrics().await;
            }
        });
    }

    async fn record_builtin_metrics(&self) {
        let health = self.get_health().await;

        self.record_metric("system_cpu_usage_percent", health.cpu_usage_percent, HashMap::new())
            .await;
        self.record_metric(
            "system_memory_usage_percent",
            health.memory_usage_percent,
            HashMap::new(),
        )
        .await;
        self.record_metric(
            "system_disk_usage_percent",
            health.disk_usage_percent,
            HashMap::new(),
        )
        .await;
        self.record_metric(
            "system_requests_per_second",
            health.requests_per_second,
            HashMap::new(),
        )
        .await;
        self.record_metric(
            "system_error_rate_percent",
            health.error_rate_percent,
            HashMap::new(),
        )
        .await;
        self.record_metric(
            "system_average_latency_ms",
            health.average_latency_ms,
            HashMap::new(),
        )
        .await;
    }

    pub async fn setup_default_alert_rules(&self) {
        let rules = vec![
            AlertRule {
                id: Uuid::new_v4(),
                name: "High CPU Usage".to_string(),
                description: "CPU usage exceeds 90%".to_string(),
                metric_name: "system_cpu_usage_percent".to_string(),
                condition: AlertCondition::GreaterThan(90.0),
                severity: AlertSeverity::Warning,
                duration_seconds: 300,
                labels: HashMap::new(),
                annotations: HashMap::new(),
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            AlertRule {
                id: Uuid::new_v4(),
                name: "Critical CPU Usage".to_string(),
                description: "CPU usage exceeds 95%".to_string(),
                metric_name: "system_cpu_usage_percent".to_string(),
                condition: AlertCondition::GreaterThan(95.0),
                severity: AlertSeverity::Critical,
                duration_seconds: 60,
                labels: HashMap::new(),
                annotations: HashMap::new(),
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            AlertRule {
                id: Uuid::new_v4(),
                name: "High Memory Usage".to_string(),
                description: "Memory usage exceeds 85%".to_string(),
                metric_name: "system_memory_usage_percent".to_string(),
                condition: AlertCondition::GreaterThan(85.0),
                severity: AlertSeverity::Warning,
                duration_seconds: 300,
                labels: HashMap::new(),
                annotations: HashMap::new(),
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            AlertRule {
                id: Uuid::new_v4(),
                name: "High Error Rate".to_string(),
                description: "Error rate exceeds 5%".to_string(),
                metric_name: "system_error_rate_percent".to_string(),
                condition: AlertCondition::GreaterThan(5.0),
                severity: AlertSeverity::Error,
                duration_seconds: 120,
                labels: HashMap::new(),
                annotations: HashMap::new(),
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            AlertRule {
                id: Uuid::new_v4(),
                name: "High Latency".to_string(),
                description: "Average latency exceeds 1000ms".to_string(),
                metric_name: "system_average_latency_ms".to_string(),
                condition: AlertCondition::GreaterThan(1000.0),
                severity: AlertSeverity::Warning,
                duration_seconds: 300,
                labels: HashMap::new(),
                annotations: HashMap::new(),
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        for rule in rules {
            self.add_alert_rule(rule).await;
        }
    }
}
