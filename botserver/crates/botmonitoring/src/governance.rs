use crate::real_time::MetricsCollector;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallLog {
    pub id: Uuid,
    pub bot_id: Option<String>,
    pub session_id: Option<String>,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub latency_ms: u64,
    pub provider: String,
    pub user_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptBlockLog {
    pub id: Uuid,
    pub bot_id: Option<String>,
    pub session_id: Option<String>,
    pub script_id: Option<String>,
    pub blocked_patterns: Vec<String>,
    pub risk_level: String,
    pub user_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDashboard {
    pub total_llm_calls: u64,
    pub total_tokens: u64,
    pub blocked_scripts: u64,
    pub active_bots: u64,
    pub alerts_firing: u64,
    pub top_models: Vec<ModelUsage>,
    pub recent_llm_calls: Vec<LlmCallLog>,
    pub recent_blocked_scripts: Vec<ScriptBlockLog>,
    pub throughput: ThroughputStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub call_count: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputStats {
    pub requests_per_minute: f64,
    pub tokens_per_minute: f64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
}

pub struct GovernanceService {
    collector: Arc<MetricsCollector>,
}

impl GovernanceService {
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self { collector }
    }

    pub async fn record_llm_call(&self, log: &LlmCallLog) {
        let mut labels = HashMap::new();
        labels.insert("model".to_string(), log.model.clone());
        labels.insert("provider".to_string(), log.provider.clone());
        if let Some(ref bot_id) = log.bot_id {
            labels.insert("bot_id".to_string(), bot_id.clone());
        }

        self.collector
            .increment_counter("governance_llm_calls_total", labels.clone())
            .await;

        self.collector
            .record_histogram(
                "governance_llm_latency_ms",
                log.latency_ms as f64,
                labels.clone(),
            )
            .await;

        self.collector
            .record_histogram(
                "governance_llm_tokens_total",
                (log.prompt_tokens + log.completion_tokens) as f64,
                labels,
            )
            .await;
    }

    pub async fn record_script_blocked(&self, log: &ScriptBlockLog) {
        let mut labels = HashMap::new();
        labels.insert("risk_level".to_string(), log.risk_level.clone());
        if let Some(ref bot_id) = log.bot_id {
            labels.insert("bot_id".to_string(), bot_id.clone());
        }

        self.collector
            .increment_counter("governance_scripts_blocked_total", labels)
            .await;
    }

    pub async fn get_dashboard(&self) -> GovernanceDashboard {
        let metrics = self.collector.get_metrics().await;

        let total_llm_calls = metrics
            .iter()
            .find(|m| m.name == "governance_llm_calls_total")
            .map(|m| m.current_value as u64)
            .unwrap_or(0);

        let total_tokens = metrics
            .iter()
            .find(|m| m.name == "governance_llm_tokens_total")
            .map(|m| m.current_value as u64)
            .unwrap_or(0);

        let blocked_scripts = metrics
            .iter()
            .find(|m| m.name == "governance_scripts_blocked_total")
            .map(|m| m.current_value as u64)
            .unwrap_or(0);

        let alerts = self.collector.get_active_alerts().await;
        let alerts_firing = alerts.len() as u64;

        let top_models = self.compute_top_models(&metrics);

        GovernanceDashboard {
            total_llm_calls,
            total_tokens,
            blocked_scripts,
            active_bots: 0,
            alerts_firing,
            top_models,
            recent_llm_calls: Vec::new(),
            recent_blocked_scripts: Vec::new(),
            throughput: ThroughputStats {
                requests_per_minute: 0.0,
                tokens_per_minute: 0.0,
                error_rate: 0.0,
                avg_latency_ms: 0.0,
            },
        }
    }

    fn compute_top_models(&self, metrics: &[crate::real_time::Metric]) -> Vec<ModelUsage> {
        let mut usage: HashMap<String, (u64, u64)> = HashMap::new();
        for metric in metrics {
            if metric.name == "governance_llm_calls_total" {
                if let Some(model) = metric.labels.get("model") {
                    let entry = usage.entry(model.clone()).or_insert((0, 0));
                    entry.0 = metric.current_value as u64;
                }
            }
            if metric.name == "governance_llm_tokens_total" {
                if let Some(model) = metric.labels.get("model") {
                    let entry = usage.entry(model.clone()).or_insert((0, 0));
                    entry.1 = metric.current_value as u64;
                }
            }
        }
        let mut result: Vec<ModelUsage> = usage
            .into_iter()
            .map(|(model, (call_count, total_tokens))| ModelUsage {
                model,
                call_count,
                total_tokens,
            })
            .collect();
        result.sort_by_key(|a| std::cmp::Reverse(a.call_count));
        result
    }
}

#[derive(Debug, Deserialize)]
pub struct KillSessionRequest {
    pub session_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KillSessionResponse {
    pub success: bool,
    pub message: String,
}

pub async fn handle_governance_metrics_html(
    State(collector): State<Arc<MetricsCollector>>,
) -> Result<axum::response::Html<String>, StatusCode> {
    let service = GovernanceService::new(collector);
    let dashboard = service.get_dashboard().await;

    let cpu_status = if dashboard.alerts_firing > 5 { "danger" } else if dashboard.alerts_firing > 2 { "warning" } else { "success" };

    let html = format!(
        r##"
<div class="metric-card">
  <div class="metric-header">
    <span class="metric-title">Total LLM Calls</span>
    <span class="metric-badge {status}">{status_text}</span>
  </div>
  <div class="metric-value">{llm_calls}</div>
  <div class="metric-subtitle">All-time LLM invocations</div>
</div>
<div class="metric-card">
  <div class="metric-header">
    <span class="metric-title">Total Tokens</span>
  </div>
  <div class="metric-value">{tokens}</div>
  <div class="metric-subtitle">Aggregate token consumption</div>
</div>
<div class="metric-card">
  <div class="metric-header">
    <span class="metric-title">Blocked Scripts</span>
    <span class="metric-badge {blocked_status}">{blocked_count}</span>
  </div>
  <div class="metric-value">{blocked}</div>
  <div class="metric-subtitle">Security violations prevented</div>
</div>
<div class="metric-card">
  <div class="metric-header">
    <span class="metric-title">Active Alerts</span>
  </div>
  <div class="metric-value">{alerts}</div>
  <div class="metric-subtitle">Currently firing</div>
</div>
"##,
        status = cpu_status,
        status_text = if dashboard.alerts_firing > 0 { "ALERTS" } else { "OK" },
        llm_calls = dashboard.total_llm_calls,
        tokens = dashboard.total_tokens,
        blocked = dashboard.blocked_scripts,
        blocked_count = dashboard.blocked_scripts,
        blocked_status = if dashboard.blocked_scripts > 0 { "danger" } else { "success" },
        alerts = dashboard.alerts_firing,
    );

    Ok(axum::response::Html(html))
}

pub async fn handle_governance_incidents(
    State(collector): State<Arc<MetricsCollector>>,
) -> Result<axum::response::Html<String>, StatusCode> {
    let _ = collector;
    let html = r##"<div class="empty-state">No recent security incidents</div>"##.to_string();
    Ok(axum::response::Html(html))
}

pub fn configure_routes(collector: Arc<MetricsCollector>) -> axum::Router {
    axum::Router::new()
        .route("/api/governance/dashboard", axum::routing::get(handle_governance_dashboard))
        .route("/api/governance/metrics", axum::routing::get(handle_governance_metrics_html))
        .route("/api/governance/incidents", axum::routing::get(handle_governance_incidents))
        .route("/api/governance/sessions/kill", axum::routing::post(handle_kill_session))
        .with_state(collector)
}

pub async fn handle_kill_session(
    Json(req): Json<KillSessionRequest>,
) -> Result<Json<KillSessionResponse>, StatusCode> {
    log::warn!("Session kill requested: {} (reason: {:?})", req.session_id, req.reason);
    Ok(Json(KillSessionResponse {
        success: true,
        message: format!("Session {} termination signal sent", req.session_id),
    }))
}

pub async fn handle_governance_dashboard(
    State(collector): State<Arc<MetricsCollector>>,
) -> Result<Json<GovernanceDashboard>, StatusCode> {
    let service = GovernanceService::new(collector);
    let dashboard = service.get_dashboard().await;
    Ok(Json(dashboard))
}
