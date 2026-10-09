pub(crate) use botlib::security::create_tls_client;
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::mpsc;
pub(crate) use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesConfig {
    pub url: String,

    pub token: String,

    pub org: String,

    pub bucket: String,

    pub batch_size: usize,

    pub flush_interval_ms: u64,

    pub verify_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub measurement: String,

    pub tags: HashMap<String, String>,

    pub fields: HashMap<String, FieldValue>,

    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldValue {
    Float(f64),
    Integer(i64),
    UnsignedInteger(u64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

pub struct TimeSeriesClient {
    config: TimeSeriesConfig,
    http_client: reqwest::Client,
    write_sender: mpsc::Sender<MetricPoint>,
}

pub struct Metrics;

#[derive(Debug, Clone)]
pub enum TimeSeriesError {
    ConnectionError(String),
    WriteError(String),
    QueryError(String),
    ConfigError(String),
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
