use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::models::{
    NewRbacUserGroup, NewRbacUserRole, RbacGroup, RbacRole, RbacUserGroup, RbacUserRole,
};
use botcore::shared::state::AppState;
use botsecurity::error_sanitizer::log_and_sanitize_str;

use crate::rbac::AssignRoleRequest;

pub async fn get_user_roles(State(state): State<Arc<AppState>>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_user_roles, rbac_roles};
        let roles: Vec<RbacRole> = rbac_user_roles::table
            .filter(rbac_user_roles::user_id.eq(user_id))
            .inner_join(rbac_roles::table)
            .filter(rbac_roles::is_active.eq(true))
            .select(RbacRole::as_select())
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;
        Ok(serde_json::json!({ "user_id": user_id, "roles": roles }))
    })
    .await;

    match result {
        Ok(Ok(data)) => Json(data).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "get_user_roles", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "get_user_roles", None)).into_response()
        }
    }
}

pub async fn assign_role_to_user(
    State(state): State<Arc<AppState>>,
    Path((user_id, role_id)): Path<(Uuid, Uuid)>,
    body: Option<Json<AssignRoleRequest>>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let expires_at = body.and_then(|b| b.expires_at);

    let result = tokio::task::spawn_blocking(move || -> Result<RbacUserRole, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_user_roles;
        let assignment = NewRbacUserRole {
            id: Uuid::new_v4(),
            user_id,
            role_id,
            expires_at,
            granted_by: None,
            granted_at: Utc::now(),
        };
        diesel::insert_into(rbac_user_roles::table)
            .values(&assignment)
            .get_result::<RbacUserRole>(&mut db_conn)
            .map_err(|e| format!("Insert error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(ur)) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "rbac",
                actor_id: Uuid::nil(),
                action: "role.assign",
                target_type: Some("user"),
                target_id: Some(user_id),
                success: true,
                details: Some(&format!("Assigned role {role_id} to user {user_id}")),
            });
            (StatusCode::CREATED, Json(ur)).into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "assign_role_to_user", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "assign_role_to_user", None)).into_response()
        }
    }
}

pub async fn remove_role_from_user(
    State(state): State<Arc<AppState>>,
    Path((user_id, role_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_user_roles;
        diesel::delete(
            rbac_user_roles::table
                .filter(rbac_user_roles::user_id.eq(user_id))
                .filter(rbac_user_roles::role_id.eq(role_id)),
        )
        .execute(&mut db_conn)
        .map_err(|e| format!("Delete error: {e}"))?;
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "rbac",
                actor_id: Uuid::nil(),
                action: "role.revoke",
                target_type: Some("user"),
                target_id: Some(user_id),
                success: true,
                details: Some(&format!("Revoked role {role_id} from user {user_id}")),
            });
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "remove_role_from_user", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "remove_role_from_user", None)).into_response()
        }
    }
}

pub async fn get_user_groups(State(state): State<Arc<AppState>>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_user_groups, rbac_groups};
        let groups: Vec<RbacGroup> = rbac_user_groups::table
            .filter(rbac_user_groups::user_id.eq(user_id))
            .inner_join(rbac_groups::table)
            .filter(rbac_groups::is_active.eq(true))
            .select(RbacGroup::as_select())
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;
        Ok(serde_json::json!({ "user_id": user_id, "groups": groups }))
    })
    .await;

    match result {
        Ok(Ok(data)) => Json(data).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "get_user_groups", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "get_user_groups", None)).into_response()
        }
    }
}

pub async fn add_user_to_group(
    State(state): State<Arc<AppState>>,
    Path((user_id, group_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<RbacUserGroup, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_user_groups;
        let membership = NewRbacUserGroup {
            id: Uuid::new_v4(),
            user_id,
            group_id,
            added_by: None,
            added_at: Utc::now(),
        };
        diesel::insert_into(rbac_user_groups::table)
            .values(&membership)
            .get_result::<RbacUserGroup>(&mut db_conn)
            .map_err(|e| format!("Insert error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(ug)) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "rbac",
                actor_id: Uuid::nil(),
                action: "group.add_user",
                target_type: Some("group"),
                target_id: Some(group_id),
                success: true,
                details: Some(&format!("Added user {user_id} to group {group_id}")),
            });
            (StatusCode::CREATED, Json(ug)).into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "add_user_to_group", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "add_user_to_group", None)).into_response()
        }
    }
}

pub async fn remove_user_from_group(
    State(state): State<Arc<AppState>>,
    Path((user_id, group_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_user_groups;
        diesel::delete(
            rbac_user_groups::table
                .filter(rbac_user_groups::user_id.eq(user_id))
                .filter(rbac_user_groups::group_id.eq(group_id)),
        )
        .execute(&mut db_conn)
        .map_err(|e| format!("Delete error: {e}"))?;
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "rbac",
                actor_id: Uuid::nil(),
                action: "group.remove_user",
                target_type: Some("group"),
                target_id: Some(group_id),
                success: true,
                details: Some(&format!("Removed user {user_id} from group {group_id}")),
            });
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "remove_user_from_group", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "remove_user_from_group", None)).into_response()
        }
    }
}
