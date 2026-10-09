pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::atomic::{AtomicU64, Ordering};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::{broadcast, RwLock};
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertStatus {
    Firing,
    Resolved,
    Acknowledged,
    Silenced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub unit: Option<String>,
    pub labels: HashMap<String, String>,
    pub current_value: f64,
    pub data_points: Vec<MetricDataPoint>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub metric_name: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub duration_seconds: u64,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    GreaterThan(f64),
    LessThan(f64),
    Equals(f64),
    NotEquals(f64),
    GreaterThanOrEqual(f64),
    LessThanOrEqual(f64),
    AbsentFor(u64),
    RateOfChange { threshold: f64, window_seconds: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub status: AlertStatus,
    pub metric_name: String,
    pub metric_value: f64,
    pub threshold: f64,
    pub message: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub active_connections: u64,
    pub requests_per_second: f64,
    pub error_rate_percent: f64,
    pub average_latency_ms: f64,
    pub uptime_seconds: u64,
    pub last_check: DateTime<Utc>,
    pub components: Vec<ComponentHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub latency_ms: Option<f64>,
    pub message: Option<String>,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MonitoringMessage {
    MetricUpdate {
        metric: Metric,
    },
    AlertFired {
        alert: Alert,
    },
    AlertResolved {
        alert_id: Uuid,
    },
    HealthUpdate {
        health: SystemHealth,
    },
    Subscribe {
        metrics: Vec<String>,
        alerts: bool,
        health: bool,
    },
    Unsubscribe {
        metrics: Vec<String>,
    },
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardQuery {
    pub metrics: Option<Vec<String>>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub metrics: Vec<Metric>,
    pub health: SystemHealth,
    pub active_alerts: Vec<Alert>,
    pub recent_alerts: Vec<Alert>,
}

pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
    alerts: Arc<RwLock<HashMap<Uuid, Alert>>>,
    alert_rules: Arc<RwLock<Vec<AlertRule>>>,
    health: Arc<RwLock<SystemHealth>>,
    broadcast_tx: broadcast::Sender<MonitoringMessage>,
    request_counter: AtomicU64,
    error_counter: AtomicU64,
    total_latency_ms: AtomicU64,
    start_time: DateTime<Utc>,
}

mod part1;
mod part2;
mod part3;
mod part4;
