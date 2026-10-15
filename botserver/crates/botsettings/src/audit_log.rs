use chrono::Utc;
use diesel::RunQueryDsl;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;

/// One administrative action to record.
///
/// The fields are named because several are the same shape (`actor_id` and
/// `target_id` are both a `Uuid`, `target_type` and `details` both optional
/// text), so positional arguments at the call sites were easy to transpose.
pub struct AuditEvent<'a> {
    pub state: &'a Arc<AppState>,
    pub event_type: &'a str,
    pub actor_id: Uuid,
    pub action: &'a str,
    pub target_type: Option<&'a str>,
    pub target_id: Option<Uuid>,
    pub success: bool,
    pub details: Option<&'a str>,
}

/// Writes one durable audit event to the `audit_log` table. Every RBAC and
/// settings mutation should call this so administrative actions leave a
/// persistent, queryable compliance trail.
pub fn record_audit_event(event: AuditEvent<'_>) {
    let AuditEvent {
        state,
        event_type,
        actor_id,
        action,
        target_type,
        target_id,
        success,
        details,
    } = event;
    let Some(mut conn) = state.conn.get().ok() else {
        log::warn!("audit: DB unavailable, event dropped ({action})");
        return;
    };

    let details_owned = details.map(|d| d.to_string());
    let event_type_owned = event_type.to_string();
    let actor_type = "user".to_string();
    let action_owned = action.to_string();
    let target_type_owned = target_type.map(|t| t.to_string());
    let risk_level = if success { "info" } else { "high" };

    let result = diesel::sql_query(
        "INSERT INTO audit_log \
         (event_type, actor_type, actor_id, action, target_type, target_id, \
          outcome_success, details, bot_id, risk_level) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind::<diesel::sql_types::Text, _>(&event_type_owned)
    .bind::<diesel::sql_types::Text, _>(&actor_type)
    .bind::<diesel::sql_types::Uuid, _>(actor_id)
    .bind::<diesel::sql_types::Text, _>(&action_owned)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(target_type_owned)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(target_id)
    .bind::<diesel::sql_types::Bool, _>(success)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(details_owned)
    .bind::<diesel::sql_types::Uuid, _>(Uuid::nil())
    .bind::<diesel::sql_types::Text, _>(&risk_level)
    .execute(&mut conn);

    if let Err(e) = result {
        log::error!("audit: failed to persist event ({action}): {e}");
    }
}

/// Reads the most recent audit events from the `audit_log` table.
pub fn list_audit_events(
    state: &Arc<AppState>,
    limit: i64,
) -> Vec<serde_json::Value> {
    let Some(mut conn) = state.conn.get().ok() else {
        return Vec::new();
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        timestamp: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Text)]
        event_type: String,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        actor_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        action: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        target_type: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
        target_id: Option<Uuid>,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        outcome_success: bool,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        details: Option<String>,
    }

    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, timestamp, event_type, actor_id, action, target_type, target_id, \
                outcome_success, details \
         FROM audit_log ORDER BY timestamp DESC LIMIT $1",
    )
    .bind::<diesel::sql_types::BigInt, _>(limit)
    .load::<Row>(&mut conn)
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "timestamp": r.timestamp,
                "event_type": r.event_type,
                "actor_id": r.actor_id,
                "action": r.action,
                "target_type": r.target_type,
                "target_id": r.target_id,
                "success": r.outcome_success,
                "details": r.details,
            })
        })
        .collect()
}
