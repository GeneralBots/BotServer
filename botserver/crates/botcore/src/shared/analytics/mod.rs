//! Metrics collection, dashboard aggregation and metrics HTTP endpoints.

pub(crate) use crate::urls::ApiUrls;
pub(crate) use crate::shared::state::AppState;
pub(crate) use axum::{ extract::{Json, Query, State}, http::StatusCode, response::IntoResponse, };
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use log::info;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MetricsCollector {
    metrics: Arc<RwLock<Vec<Metric>>>,
    aggregates: Arc<RwLock<HashMap<String, f64>>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardData {
    pub total_users: i64,
    pub active_users: i64,
    pub total_messages: i64,
    pub total_sessions: i64,
    pub storage_used_gb: f64,
    pub api_calls_per_minute: f64,
    pub error_rate: f64,
    pub response_time_p95: f64,
    pub charts: Vec<ChartData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChartData {
    pub title: String,
    pub chart_type: String,
    pub labels: Vec<String>,
    pub datasets: Vec<DataSet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataSet {
    pub label: String,
    pub data: Vec<f64>,
    pub color: String,
}

#[derive(Debug, Deserialize)]
pub struct MetricQuery {
    pub name: String,
    pub window_minutes: Option<i64>,
    pub aggregation: Option<String>,
}

mod collector;
mod collect;
mod dashboard;
mod endpoints;
mod activity;

pub use collect::*;
pub use dashboard::*;
pub use endpoints::*;
pub use activity::*;
