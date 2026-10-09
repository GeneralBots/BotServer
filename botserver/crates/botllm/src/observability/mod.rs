pub(crate) use chrono::{DateTime, Utc};
pub(crate) use rhai::{Dynamic, Engine, Map};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::atomic::{AtomicU64, Ordering};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::info;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequestMetrics {
    pub request_id: Uuid,

    pub session_id: Uuid,

    pub bot_id: Uuid,

    pub model: String,

    pub request_type: RequestType,

    pub input_tokens: u64,

    pub output_tokens: u64,

    pub total_tokens: u64,

    pub latency_ms: u64,

    pub ttft_ms: Option<u64>,

    pub cached: bool,

    pub success: bool,

    pub error: Option<String>,

    pub estimated_cost: f64,

    pub timestamp: DateTime<Utc>,

    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum RequestType {
    #[default]
    Chat,
    Completion,
    Embedding,
    Rerank,
    Moderation,
    ImageGeneration,
    AudioTranscription,
    AudioGeneration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AggregatedMetrics {
    pub period_start: DateTime<Utc>,

    pub period_end: DateTime<Utc>,

    pub total_requests: u64,

    pub successful_requests: u64,

    pub failed_requests: u64,

    pub cache_hits: u64,

    pub cache_misses: u64,

    pub total_input_tokens: u64,

    pub total_output_tokens: u64,

    pub total_tokens: u64,

    pub total_cost: f64,

    pub avg_latency_ms: f64,

    pub p50_latency_ms: f64,

    pub p95_latency_ms: f64,

    pub p99_latency_ms: f64,

    pub max_latency_ms: u64,

    pub min_latency_ms: u64,

    pub requests_by_model: HashMap<String, u64>,

    pub tokens_by_model: HashMap<String, u64>,

    pub cost_by_model: HashMap<String, f64>,

    pub requests_by_type: HashMap<String, u64>,

    pub errors_by_type: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model: String,

    pub input_cost_per_1k: f64,

    pub output_cost_per_1k: f64,

    pub cost_per_request: f64,

    pub is_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub daily_limit: f64,

    pub monthly_limit: f64,

    pub alert_threshold: f64,

    pub daily_spend: f64,

    pub monthly_spend: f64,

    pub daily_reset_date: DateTime<Utc>,

    pub monthly_reset_date: DateTime<Utc>,

    pub daily_alert_sent: bool,

    pub monthly_alert_sent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub id: Uuid,

    pub parent_id: Option<Uuid>,

    pub trace_id: Uuid,

    pub name: String,

    pub component: String,

    pub event_type: TraceEventType,

    pub duration_ms: Option<u64>,

    pub start_time: DateTime<Utc>,

    pub end_time: Option<DateTime<Utc>>,

    pub attributes: HashMap<String, String>,

    pub status: TraceStatus,

    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceEventType {
    Span,
    Event,
    Log,
    Metric,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TraceStatus {
    Ok,
    Error,
    #[default]
    InProgress,
}


#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub enabled: bool,

    pub metrics_interval: u64,

    pub cost_tracking: bool,

    pub trace_enabled: bool,

    pub trace_sample_rate: f64,

    pub budget_daily: f64,

    pub budget_monthly: f64,

    pub alert_threshold: f64,

    pub model_pricing: HashMap<String, ModelPricing>,
}

#[derive(Debug)]
pub struct ObservabilityManager {
    config: ObservabilityConfig,

    metrics_buffer: Arc<RwLock<Vec<LLMRequestMetrics>>>,

    current_metrics: Arc<RwLock<AggregatedMetrics>>,

    budget: Arc<RwLock<Budget>>,

    trace_buffer: Arc<RwLock<Vec<TraceEvent>>>,

    request_count: AtomicU64,
    token_count: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    error_count: AtomicU64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub daily_limit: f64,
    pub daily_spend: f64,
    pub daily_remaining: f64,
    pub daily_percentage: f64,
    pub monthly_limit: f64,
    pub monthly_spend: f64,
    pub monthly_remaining: f64,
    pub monthly_percentage: f64,
    pub daily_exceeded: bool,
    pub monthly_exceeded: bool,
    pub near_daily_limit: bool,
    pub near_monthly_limit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetCheckResult {
    Ok,
    NearDailyLimit,
    NearMonthlyLimit,
    DailyExceeded,
    MonthlyExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickStats {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub error_count: u64,
    pub cache_hit_rate: f64,
    pub error_rate: f64,
}

pub const OBSERVABILITY_SCHEMA: &str = r"
-- LLM request metrics
CREATE TABLE IF NOT EXISTS llm_metrics (
    id UUID PRIMARY KEY,
    request_id UUID NOT NULL,
    session_id UUID NOT NULL,
    bot_id UUID NOT NULL,
    model VARCHAR(200) NOT NULL,
    request_type VARCHAR(50) NOT NULL,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    latency_ms BIGINT NOT NULL DEFAULT 0,
    ttft_ms BIGINT,
    cached BOOLEAN NOT NULL DEFAULT false,
    success BOOLEAN NOT NULL DEFAULT true,
    error TEXT,
    estimated_cost DOUBLE PRECISION NOT NULL DEFAULT 0,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    metadata JSONB NOT NULL DEFAULT '{}'
);

-- Aggregated metrics (hourly rollup)
CREATE TABLE IF NOT EXISTS llm_metrics_hourly (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL,
    hour TIMESTAMP WITH TIME ZONE NOT NULL,
    total_requests BIGINT NOT NULL DEFAULT 0,
    successful_requests BIGINT NOT NULL DEFAULT 0,
    failed_requests BIGINT NOT NULL DEFAULT 0,
    cache_hits BIGINT NOT NULL DEFAULT 0,
    cache_misses BIGINT NOT NULL DEFAULT 0,
    total_input_tokens BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    total_cost DOUBLE PRECISION NOT NULL DEFAULT 0,
    avg_latency_ms DOUBLE PRECISION NOT NULL DEFAULT 0,
    p50_latency_ms DOUBLE PRECISION NOT NULL DEFAULT 0,
    p95_latency_ms DOUBLE PRECISION NOT NULL DEFAULT 0,
    p99_latency_ms DOUBLE PRECISION NOT NULL DEFAULT 0,
    max_latency_ms BIGINT NOT NULL DEFAULT 0,
    min_latency_ms BIGINT NOT NULL DEFAULT 0,
    requests_by_model JSONB NOT NULL DEFAULT '{}',
    tokens_by_model JSONB NOT NULL DEFAULT '{}',
    cost_by_model JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    UNIQUE(bot_id, hour)
);

-- Budget tracking
CREATE TABLE IF NOT EXISTS llm_budget (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL UNIQUE,
    daily_limit DOUBLE PRECISION NOT NULL DEFAULT 100,
    monthly_limit DOUBLE PRECISION NOT NULL DEFAULT 2000,
    alert_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.8,
    daily_spend DOUBLE PRECISION NOT NULL DEFAULT 0,
    monthly_spend DOUBLE PRECISION NOT NULL DEFAULT 0,
    daily_reset_date DATE NOT NULL DEFAULT CURRENT_DATE,
    monthly_reset_date DATE NOT NULL DEFAULT DATE_TRUNC('month', CURRENT_DATE),
    daily_alert_sent BOOLEAN NOT NULL DEFAULT false,
    monthly_alert_sent BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Trace events
CREATE TABLE IF NOT EXISTS llm_traces (
    id UUID PRIMARY KEY,
    parent_id UUID,
    trace_id UUID NOT NULL,
    name VARCHAR(200) NOT NULL,
    component VARCHAR(100) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    duration_ms BIGINT,
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time TIMESTAMP WITH TIME ZONE,
    attributes JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(50) NOT NULL DEFAULT 'in_progress',
    error TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_llm_metrics_bot_id ON llm_metrics(bot_id);
CREATE INDEX IF NOT EXISTS idx_llm_metrics_session_id ON llm_metrics(session_id);
CREATE INDEX IF NOT EXISTS idx_llm_metrics_timestamp ON llm_metrics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_llm_metrics_model ON llm_metrics(model);

CREATE INDEX IF NOT EXISTS idx_llm_metrics_hourly_bot_id ON llm_metrics_hourly(bot_id);
CREATE INDEX IF NOT EXISTS idx_llm_metrics_hourly_hour ON llm_metrics_hourly(hour DESC);

CREATE INDEX IF NOT EXISTS idx_llm_traces_trace_id ON llm_traces(trace_id);
CREATE INDEX IF NOT EXISTS idx_llm_traces_start_time ON llm_traces(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_llm_traces_component ON llm_traces(component);
";

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

use part4::*;
pub use part7::*;
