pub(crate) use chrono::{DateTime, Utc};
pub(crate) use rand::Rng;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependency {
    pub parent_service: String,
    pub child_service: String,
    pub call_count: u64,
    pub error_count: u64,
    pub avg_duration_us: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SpanKind {
    Server,
    Client,
    Producer,
    Consumer,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub trace_flags: u8,
    pub trace_state: HashMap<String, String>,
    pub baggage: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub kind: SpanKind,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_us: Option<i64>,
    pub status: SpanStatus,
    pub status_message: Option<String>,
    pub attributes: HashMap<String, AttributeValue>,
    pub events: Vec<SpanEvent>,
    pub links: Vec<SpanLink>,
    pub resource: ResourceAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    StringArray(Vec<String>),
    IntArray(Vec<i64>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub attributes: HashMap<String, AttributeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLink {
    pub trace_id: String,
    pub span_id: String,
    pub attributes: HashMap<String, AttributeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAttributes {
    pub service_name: String,
    pub service_version: String,
    pub service_instance_id: String,
    pub host_name: Option<String>,
    pub host_type: Option<String>,
    pub os_type: Option<String>,
    pub deployment_environment: Option<String>,
    pub custom: HashMap<String, AttributeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: String,
    pub spans: Vec<Span>,
    pub root_span_id: Option<String>,
    pub service_count: u32,
    pub span_count: u32,
    pub duration_us: Option<i64>,
    pub has_errors: bool,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceQuery {
    pub service_name: Option<String>,
    pub operation_name: Option<String>,
    pub min_duration_us: Option<i64>,
    pub max_duration_us: Option<i64>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub tags: HashMap<String, String>,
    pub has_errors: Option<bool>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStatistics {
    pub total_traces: u64,
    pub total_spans: u64,
    pub error_rate: f32,
    pub avg_duration_us: f64,
    pub p50_duration_us: i64,
    pub p90_duration_us: i64,
    pub p95_duration_us: i64,
    pub p99_duration_us: i64,
    pub spans_per_second: f32,
    pub service_breakdown: HashMap<String, ServiceStats>,
    pub operation_breakdown: HashMap<String, OperationStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStats {
    pub span_count: u64,
    pub error_count: u64,
    pub avg_duration_us: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    pub span_count: u64,
    pub error_count: u64,
    pub avg_duration_us: f64,
    pub min_duration_us: i64,
    pub max_duration_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingConfig {
    pub strategy: SamplingStrategy,
    pub rate: f32,
    pub max_traces_per_second: u32,
    pub service_overrides: HashMap<String, f32>,
    pub operation_overrides: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SamplingStrategy {
    Always,
    Never,
    Probabilistic,
    RateLimiting,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExporterConfig {
    pub exporter_type: ExporterType,
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub batch_size: u32,
    pub flush_interval_ms: u32,
    pub timeout_ms: u32,
    pub compression: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExporterType {
    Otlp,
    Jaeger,
    Zipkin,
    Console,
    None,
}

pub struct SpanBuilder {
    operation_name: String,
    service_name: String,
    kind: SpanKind,
    parent_context: Option<TraceContext>,
    attributes: HashMap<String, AttributeValue>,
    links: Vec<SpanLink>,
    start_time: Option<DateTime<Utc>>,
}

pub struct ActiveSpan {
    span: Span,
    context: TraceContext,
    tracer: DistributedTracingService,
}

#[derive(Clone)]
pub struct DistributedTracingService {
    spans: Arc<RwLock<HashMap<String, Vec<Span>>>>,
    sampling_config: Arc<RwLock<SamplingConfig>>,
    exporter_config: Arc<RwLock<ExporterConfig>>,
    resource: Arc<ResourceAttributes>,
    span_buffer: Arc<RwLock<Vec<Span>>>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;
mod part11;
mod part12;

use part1::*;
use part12::*;
