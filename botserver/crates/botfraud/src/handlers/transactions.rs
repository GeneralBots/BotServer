//! Transaction endpoints (merged from `botapps/fraud.rs`).

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use super::state::{resolve_branch, FraudState};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FraudTransaction {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub user_id: Uuid,
    pub amount: String,
    pub currency: String,
    pub status: String,
    pub risk_score: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_transactions(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<FraudTransaction>>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)] branch_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)] user_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Numeric)] amount: rust_decimal::Decimal,
        #[diesel(sql_type = diesel::sql_types::Text)] currency: String,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)] risk_score: Option<i32>,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<chrono::Utc>,
    }
    // Ensure fraud_transactions table exists
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS fraud_transactions (
            id UUID PRIMARY KEY, branch_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
            user_id UUID NOT NULL DEFAULT gen_random_uuid(),
            amount NUMERIC(18,4) NOT NULL DEFAULT 0, currency VARCHAR(8) NOT NULL DEFAULT 'BRL',
            status VARCHAR(30) NOT NULL DEFAULT 'pending', risk_score INTEGER,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())"
    ).execute(&mut conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, branch_id, user_id, amount, currency, status, risk_score, created_at
         FROM fraud_transactions WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 500",
    ).bind::<diesel::sql_types::Uuid, _>(branch).load(&mut conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(|r| FraudTransaction {
        id: r.id, branch_id: r.branch_id, user_id: r.user_id, amount: r.amount.to_string(),
        currency: r.currency, status: r.status, risk_score: r.risk_score, created_at: r.created_at,
    }).collect()))
}

pub async fn create_transaction(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS fraud_transactions (
            id UUID PRIMARY KEY, branch_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
            user_id UUID NOT NULL DEFAULT gen_random_uuid(),
            amount NUMERIC(18,4) NOT NULL DEFAULT 0, currency VARCHAR(8) NOT NULL DEFAULT 'BRL',
            status VARCHAR(30) NOT NULL DEFAULT 'pending', risk_score INTEGER,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW())"
    ).execute(&mut conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let id = Uuid::new_v4();
    diesel::sql_query(
        "INSERT INTO fraud_transactions (id, branch_id, user_id, amount, currency, status, created_at)
         VALUES ($1, $2, $3, $4, $5, 'pending', NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Uuid, _>(Uuid::parse_str(payload.get("user_id").and_then(|v| v.as_str()).unwrap_or("00000000-0000-0000-0000-000000000000")).unwrap_or(Uuid::nil()))
    .bind::<diesel::sql_types::Numeric, _>(rust_decimal::Decimal::new((payload.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0) as i64, 2))
    .bind::<diesel::sql_types::Text, _>(payload.get("currency").and_then(|v| v.as_str()).unwrap_or("BRL"))
    .execute(&mut conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"id": id, "status": "pending"})))
}
