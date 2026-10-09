//! Aggregate statistics endpoint.

use crate::types::*;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use super::rows::CountRow;
use super::state::{resolve_branch, FraudState};

pub async fn stats(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
) -> Result<Json<FraudStats>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let total = count_sql(&mut conn, "SELECT COUNT(*) FROM fraud_events WHERE branch_id = $1", branch).unwrap_or(0);
    let blocked = count_sql(&mut conn, "SELECT COUNT(*) FROM fraud_events WHERE action_taken = 'block' AND branch_id = $1", branch).unwrap_or(0);
    let flagged = count_sql(&mut conn, "SELECT COUNT(*) FROM fraud_events WHERE action_taken = 'flag' AND branch_id = $1", branch).unwrap_or(0);
    let reviewed = count_sql(&mut conn, "SELECT COUNT(*) FROM fraud_events WHERE reviewed_at IS NOT NULL AND branch_id = $1", branch).unwrap_or(0);
    let high_risk = count_sql(&mut conn, "SELECT COUNT(*) FROM fraud_events WHERE risk_level IN ('high','critical') AND branch_id = $1", branch).unwrap_or(0);
    let rules_active = count_sql(&mut conn, "SELECT COUNT(*) FROM fraud_rules WHERE is_active = true AND branch_id = $1", branch).unwrap_or(0);

    Ok(Json(FraudStats {
        total_events: total,
        blocked_count: blocked,
        flagged_count: flagged,
        reviewed_count: reviewed,
        high_risk_count: high_risk,
        rules_active,
    }))
}

pub fn count_sql(
    conn: &mut PgConnection,
    sql: &str,
    branch: Uuid,
) -> Result<i64, diesel::result::Error> {
    diesel::sql_query(sql)
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .get_result::<CountRow>(conn)
        .map(|r| r.count)
}
