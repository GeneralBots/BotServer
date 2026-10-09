//! Router wiring and the assessment endpoint.

use crate::types::*;
use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};
use std::sync::Arc;

use super::rule_handlers::{
    add_blocklist, create_rule, list_blocklist, list_events, list_rules, remove_blocklist,
    toggle_rule,
};
use super::state::{resolve_branch, FraudState};
use super::stats::stats;
use super::transactions::{create_transaction, list_transactions};

pub fn configure_fraud_routes() -> Router<Arc<FraudState>> {
    Router::new()
        .route("/api/fraud/assess", axum::routing::post(assess))
        .route("/api/fraud/transactions", get(list_transactions).post(create_transaction))
        .route("/api/fraud/rules", get(list_rules).post(create_rule))
        .route("/api/fraud/rules/:id", axum::routing::put(toggle_rule))
        .route("/api/fraud/events", get(list_events))
        .route("/api/fraud/blocklist", get(list_blocklist).post(add_blocklist))
        .route("/api/fraud/blocklist/:id", axum::routing::delete(remove_blocklist))
        .route("/api/fraud/stats", get(stats))
}

async fn assess(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
    payload: Json<FraudAssessmentRequest>,
) -> Result<Json<FraudAssessmentResult>, axum::http::StatusCode> {
    // Tenant is resolved exclusively from the JWT claims (issue #734); any
    // branch_id sent by the client is ignored because it is untrusted input.
    let branch = resolve_branch(&headers);
    let result = state.engine.assess(branch, &payload.0).await;
    Ok(Json(result))
}
