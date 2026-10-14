pub mod alert_routes;
pub mod alerts_panel;
pub mod governance;
pub mod health_panel;
pub mod handlers;
pub mod panels;
pub mod quick;
pub mod log_stream;
pub mod metrics_panel;
pub mod real_time;
pub mod resources_panel;
pub mod tracing;

use axum::{routing::{delete, get, patch, post}, Router};
use std::sync::Arc;

pub use real_time::{HealthStatus, MetricsCollector};
pub use tracing::{DistributedTracingService, TraceContext};

#[derive(Debug, Clone)]
pub struct DependencyStatus {
    pub name: &'static str,
    pub host: String,
    pub healthy: bool,
    pub latency_ms: f64,
}

pub trait MonitoringState: Send + Sync + 'static {
    fn active_session_count(&self) -> usize;
    fn is_db_healthy(&self) -> bool;
    fn metrics_collector(&self) -> Arc<MetricsCollector> {
        Arc::new(MetricsCollector::new())
    }
    fn dependencies(&self) -> Vec<DependencyStatus> {
        Vec::new()
    }
}

pub fn health_status_class(status: &HealthStatus) -> &'static str {
    match status {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Degraded => "degraded",
        HealthStatus::Unhealthy => "unhealthy",
        HealthStatus::Unknown => "unknown",
    }
}

pub fn format_uptime_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

pub trait MonitoringUrls: Send + Sync + 'static {
    fn monitoring_dashboard() -> &'static str;
    fn monitoring_services() -> &'static str;
    fn monitoring_resources() -> &'static str;
    fn monitoring_logs() -> &'static str;
    fn monitoring_llm() -> &'static str;
    fn monitoring_health() -> &'static str;
    fn monitoring_timestamp() -> &'static str;
    fn monitoring_bots() -> &'static str;
    fn monitoring_services_status() -> &'static str;
    fn monitoring_resources_bars() -> &'static str;
    fn monitoring_activity_latest() -> &'static str;
    fn monitoring_metric_sessions() -> &'static str;
    fn monitoring_metric_messages() -> &'static str;
    fn monitoring_metric_response_time() -> &'static str;
    fn monitoring_trend_sessions() -> &'static str;
    fn monitoring_rate_messages() -> &'static str;
    fn monitoring_sessions_panel() -> &'static str;
    fn monitoring_messages_panel() -> &'static str;
}

pub struct DefaultMonitoringUrls;

impl MonitoringUrls for DefaultMonitoringUrls {
    fn monitoring_dashboard() -> &'static str { "/api/monitoring/dashboard" }
    fn monitoring_services() -> &'static str { "/api/monitoring/services" }
    fn monitoring_resources() -> &'static str { "/api/monitoring/resources" }
    fn monitoring_logs() -> &'static str { "/api/monitoring/logs" }
    fn monitoring_llm() -> &'static str { "/api/monitoring/llm" }
    fn monitoring_health() -> &'static str { "/api/monitoring/health" }
    fn monitoring_timestamp() -> &'static str { "/api/monitoring/timestamp" }
    fn monitoring_bots() -> &'static str { "/api/monitoring/bots" }
    fn monitoring_services_status() -> &'static str { "/api/monitoring/services/status" }
    fn monitoring_resources_bars() -> &'static str { "/api/monitoring/resources/bars" }
    fn monitoring_activity_latest() -> &'static str { "/api/monitoring/activity/latest" }
    fn monitoring_metric_sessions() -> &'static str { "/api/monitoring/metric/sessions" }
    fn monitoring_metric_messages() -> &'static str { "/api/monitoring/metric/messages" }
    fn monitoring_metric_response_time() -> &'static str { "/api/monitoring/metric/response_time" }
    fn monitoring_trend_sessions() -> &'static str { "/api/monitoring/trend/sessions" }
    fn monitoring_rate_messages() -> &'static str { "/api/monitoring/rate/messages" }
    fn monitoring_sessions_panel() -> &'static str { "/api/monitoring/sessions" }
    fn monitoring_messages_panel() -> &'static str { "/api/monitoring/messages" }
}

pub fn configure_governance_routes(collector: Arc<MetricsCollector>) -> Router {
    governance::configure_routes(collector)
}

pub fn configure<S: MonitoringState, U: MonitoringUrls>() -> Router<Arc<S>> {
    Router::new()
        .route(U::monitoring_dashboard(), get(handlers::dashboard::<S>))
        .route(U::monitoring_services(), get(handlers::services::<S>))
        .route(U::monitoring_resources(), get(handlers::resources::<S>))
        .route(U::monitoring_logs(), get(handlers::logs::<S>))
        .route(U::monitoring_llm(), get(handlers::llm_metrics::<S>))
        .route(U::monitoring_health(), get(handlers::health::<S>))
        .route(U::monitoring_timestamp(), get(panels::timestamp::<S>))
        .route(U::monitoring_bots(), get(panels::bots::<S>))
        .route(U::monitoring_services_status(), get(panels::services_status::<S>))
        .route(U::monitoring_resources_bars(), get(panels::resources_bars::<S>))
        .route(U::monitoring_activity_latest(), get(panels::activity_latest::<S>))
        .route(U::monitoring_metric_sessions(), get(panels::metric_sessions::<S>))
        .route(U::monitoring_metric_messages(), get(panels::metric_messages::<S>))
        .route(U::monitoring_metric_response_time(), get(panels::metric_response_time::<S>))
        .route(U::monitoring_trend_sessions(), get(panels::trend_sessions::<S>))
        .route(U::monitoring_rate_messages(), get(panels::rate_messages::<S>))
        .route(U::monitoring_sessions_panel(), get(panels::sessions_panel::<S>))
        .route(U::monitoring_messages_panel(), get(panels::messages_panel::<S>))
        .route("/api/monitoring/health/status", get(health_panel::health_status::<S, U>))
        .route("/api/monitoring/health/overview", get(health_panel::overview::<S, U>))
        .route("/api/monitoring/health/uptime", get(health_panel::uptime::<S, U>))
        .route("/api/monitoring/health/uptime-percent", get(health_panel::uptime_percent::<S, U>))
        .route("/api/monitoring/health/last-incident", get(health_panel::last_incident::<S, U>))
        .route("/api/monitoring/health/response-time", get(health_panel::response_time::<S, U>))
        .route("/api/monitoring/health/checks", get(health_panel::checks::<S, U>))
        .route("/api/monitoring/health/dependencies", get(health_panel::dependencies::<S, U>))
        .route("/api/monitoring/health/uptime-history", get(health_panel::uptime_history::<S, U>))
        .route("/api/monitoring/health/incidents", get(health_panel::incidents::<S, U>))
        .route("/api/monitoring/system/status", get(metrics_panel::system_status::<S, U>))
        .route("/api/monitoring/metrics", get(metrics_panel::metrics_table::<S, U>))
        .route("/api/monitoring/metrics/last-sync", get(metrics_panel::last_sync::<S, U>))
        .route("/api/monitoring/logs/services", get(metrics_panel::logs_services::<S, U>))
        .route("/api/monitoring/resources/cpu", get(resources_panel::cpu_card::<S, U>))
        .route("/api/monitoring/resources/memory", get(resources_panel::memory_card::<S, U>))
        .route("/api/monitoring/resources/disk", get(resources_panel::disk_card::<S, U>))
        .route("/api/monitoring/resources/network", get(resources_panel::network_card::<S, U>))
        .route("/api/monitoring/resources/disk/partitions", get(resources_panel::disk_partitions::<S, U>))
        .route("/api/monitoring/resources/network/interfaces", get(resources_panel::network_interfaces::<S, U>))
        .route("/api/monitoring/resources/processes", get(resources_panel::processes::<S, U>))
        .route("/api/monitoring/resources/system", get(resources_panel::system_info::<S, U>))
        .route("/api/monitoring/charts/cpu", get(resources_panel::cpu_chart::<S, U>))
        .route("/api/monitoring/charts/memory", get(resources_panel::memory_chart::<S, U>))
        .route("/api/monitoring/alerts", get(alert_routes::list_alerts::<S>))
        .route("/api/monitoring/alerts/count", get(alerts_panel::alert_count::<S>))
        .route("/api/monitoring/alerts/summary", get(alerts_panel::alert_summary::<S>))
        .route("/api/monitoring/alerts/active", get(alerts_panel::active_alerts::<S>))
        .route("/api/monitoring/alerts/history", get(alerts_panel::alert_history::<S>))
        .route("/api/monitoring/alerts/:id", get(alert_routes::get_alert::<S>))
        .route("/api/monitoring/alerts/:id/acknowledge", post(alert_routes::acknowledge_alert::<S>))
        .route("/api/monitoring/alerts/:id/silence", post(alert_routes::silence_alert::<S>))
        .route("/api/monitoring/alerts/acknowledge-all", post(alert_routes::acknowledge_all_alerts::<S>))
        .route("/api/monitoring/alerts/rules", get(alert_routes::list_rules::<S>))
        .route("/api/monitoring/alerts/rules/:id", get(alert_routes::get_rule::<S>))
        .route("/api/monitoring/alerts/rules/:id", patch(alert_routes::update_rule::<S>))
        .route("/api/monitoring/alerts/rules/:id", delete(alert_routes::delete_rule::<S>))
        .route("/api/monitoring/alerts/history/export", get(alert_routes::export_history::<S>))
        .route("/api/monitoring/export", get(alert_routes::export_monitoring_data::<S>))
        .route("/api/monitoring/quick/cpu", get(quick::quick_cpu::<S>))
        .route("/api/monitoring/quick/memory", get(quick::quick_memory::<S>))
        .route("/api/monitoring/quick/disk", get(quick::quick_disk::<S>))
        .route("/api/monitoring/quick/network", get(quick::quick_network::<S>))
        .route("/api/monitoring/quick/requests", get(quick::quick_requests::<S>))
        .route("/ws/logs", get(log_stream::ws_logs::<S>))
}

struct ServiceChecker {
    cache: std::sync::Mutex<std::collections::HashMap<&'static str, (bool, std::time::Instant)>>,
}

impl ServiceChecker {
    fn new() -> Self {
        Self {
            cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn check(&self, name: &'static str, url: Option<String>, default_port: u16) -> bool {
        let now = std::time::Instant::now();
        if let Ok(cache) = self.cache.lock() {
            if let Some((cached, at)) = cache.get(name) {
                if now.duration_since(*at).as_secs() < 15 {
                    return *cached;
                }
            }
        }

        let result = url
            .and_then(|u| service_host_port(&u, default_port))
            .map(|(host, port)| tcp_connect(&host, port))
            .unwrap_or(false);

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(name, (result, now));
        }
        result
    }
}

pub fn host_port_from_url(url: &str, default_port: u16) -> Option<(String, u16)> {
    service_host_port(url, default_port)
}

fn service_host_port(url: &str, default_port: u16) -> Option<(String, u16)> {
    let trimmed = url.trim();
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let host_part = without_scheme.split(['/', '?', '@']).next().unwrap_or(without_scheme);
    let host_part = if host_part.contains('@') {
        host_part.rsplit_once('@').map(|(_, h)| h).unwrap_or(host_part)
    } else {
        host_part
    };
    if host_part.is_empty() {
        return None;
    }
    let (host, port) = match host_part.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
            (h.to_string(), p.parse::<u16>().unwrap_or(default_port))
        }
        _ => (host_part.to_string(), default_port),
    };
    if host.is_empty() {
        return None;
    }
    Some((host, port))
}

fn tcp_connect(host: &str, port: u16) -> bool {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr: SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(750)).is_ok()
}

fn check_postgres() -> bool {
    static CHECKER: std::sync::OnceLock<ServiceChecker> = std::sync::OnceLock::new();
    let checker = CHECKER.get_or_init(ServiceChecker::new);
    let url = std::env::var("DATABASE_URL").ok();
    checker.check("postgres", url, 5432)
}

fn check_redis() -> bool {
    static CHECKER: std::sync::OnceLock<ServiceChecker> = std::sync::OnceLock::new();
    let checker = CHECKER.get_or_init(ServiceChecker::new);
    let url = std::env::var("CACHE_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .or_else(|_| std::env::var("VALKEY_URL"))
        .ok();
    checker.check("redis", url, 6379)
}

fn check_minio() -> bool {
    static CHECKER: std::sync::OnceLock<ServiceChecker> = std::sync::OnceLock::new();
    let checker = CHECKER.get_or_init(ServiceChecker::new);
    let url = std::env::var("MINIO_URL")
        .or_else(|_| std::env::var("S3_ENDPOINT"))
        .or_else(|_| std::env::var("MINIO_ENDPOINT"))
        .ok();
    checker.check("minio", url, 9000)
}

fn check_llm() -> bool {
    static CHECKER: std::sync::OnceLock<ServiceChecker> = std::sync::OnceLock::new();
    let checker = CHECKER.get_or_init(ServiceChecker::new);
    let url = std::env::var("LLM_URL")
        .or_else(|_| std::env::var("OLLAMA_HOST"))
        .ok();
    checker.check("llm", url, 8081)
}

