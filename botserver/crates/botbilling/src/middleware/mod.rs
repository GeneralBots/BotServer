pub(crate) use axum::{ body::Body, extract::State, http::{header::HeaderValue, Request, StatusCode}, middleware::Next, response::{IntoResponse, Response}, Json, };
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;
pub(crate) use crate::quotas::{QuotaAction, QuotaManager};
pub(crate) use crate::UsageMetric;

#[derive(Clone)]
pub struct QuotaMiddlewareState {
    pub quota_manager: Arc<QuotaManager>,
    pub enabled: Arc<RwLock<bool>>,
}

#[derive(Clone)]
pub struct OrganizationContext {
    pub organization_id: Uuid,
    pub user_id: Option<Uuid>,
    pub plan_id: Option<String>,
}

struct QuotaExceededResponse {
    message: String,
}

struct RateLimitExceededResponse {
    message: String,
}

struct MessageQuotaExceededResponse {
    message: String,
}

struct StorageQuotaExceededResponse {
    message: String,
    current_usage: u64,
    limit: Option<u64>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

pub use part1::*;
use part2::*;
pub use part3::*;
pub use part7::*;
