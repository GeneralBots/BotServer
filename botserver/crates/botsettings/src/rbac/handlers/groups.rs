use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::models::{
    NewRbacGroup, NewRbacGroupRole, RbacGroup, RbacGroupRole, RbacRole,
};
use botcore::shared::state::AppState;
use botsecurity::error_sanitizer::log_and_sanitize_str;
use diesel::prelude::*;

use super::super::CreateGroupRequest;

pub async fn list_groups(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<RbacGroup>, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_groups;
        rbac_groups::table
            .filter(rbac_groups::is_active.eq(true))
            .order(rbac_groups::display_name.asc())
            .load::<RbacGroup>(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(groups)) => Json(serde_json::json!({ "groups": groups })).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "list_groups", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "list_groups", None)).into_response()
        }
    }
}

pub async fn get_group(State(state): State<Arc<AppState>>, Path(group_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<RbacGroup, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_groups;
        rbac_groups::table
            .find(group_id)
            .first::<RbacGroup>(&mut db_conn)
            .map_err(|e| format!("Group not found: {e}"))
    })
    .await;

    match result {
        Ok(Ok(group)) => Json(group).into_response(),
        Ok(Err(e)) => {
            (StatusCode::NOT_FOUND, log_and_sanitize_str(&e, "get_group", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "get_group", None)).into_response()
        }
    }
}

pub async fn create_group(State(state): State<Arc<AppState>>, Json(req): Json<CreateGroupRequest>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let now = Utc::now();
    let new_group = NewRbacGroup {
        id: Uuid::new_v4(),
        name: req.name.to_lowercase().replace(' ', "_"),
        display_name: req.display_name,
        description: req.description,
        parent_group_id: req.parent_group_id,
        is_active: true,
        created_by: None,
        created_at: now,
        updated_at: now,
    };

    let result = tokio::task::spawn_blocking(move || -> Result<RbacGroup, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_groups;
        diesel::insert_into(rbac_groups::table)
            .values(&new_group)
            .get_result::<RbacGroup>(&mut db_conn)
            .map_err(|e| format!("Insert error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(group)) => {
            log::info!("Created group: {} ({})", group.display_name, group.id);
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "rbac",
                actor_id: Uuid::nil(),
                action: "group.create",
                target_type: Some("group"),
                target_id: Some(group.id),
                success: true,
                details: Some(&format!("Created group '{}'", group.name)),
            });
            (StatusCode::CREATED, Json(group)).into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "create_group", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "create_group", None)).into_response()
        }
    }
}

pub async fn update_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let now = Utc::now();
    let result = tokio::task::spawn_blocking(move || -> Result<RbacGroup, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_groups;
        let _group: RbacGroup = rbac_groups::table
            .find(group_id)
            .first(&mut db_conn)
            .map_err(|e| format!("Group not found: {e}"))?;
        diesel::update(rbac_groups::table.find(group_id))
            .set((
                rbac_groups::display_name.eq(&req.display_name),
                rbac_groups::description.eq(&req.description),
                rbac_groups::name.eq(req.name.to_lowercase().replace(' ', "_")),
                rbac_groups::parent_group_id.eq(req.parent_group_id),
                rbac_groups::updated_at.eq(now),
            ))
            .get_result::<RbacGroup>(&mut db_conn)
            .map_err(|e| format!("Update error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(group)) => Json(group).into_response(),
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "update_group", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "update_group", None)).into_response()
        }
    }
}

pub async fn delete_group(State(state): State<Arc<AppState>>, Path(group_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_groups;
        diesel::update(rbac_groups::table.find(group_id))
            .set(rbac_groups::is_active.eq(false))
            .execute(&mut db_conn)
            .map_err(|e| format!("Delete error: {e}"))?;
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "delete_group", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "delete_group", None)).into_response()
        }
    }
}

pub async fn get_group_roles(State(state): State<Arc<AppState>>, Path(group_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_group_roles, rbac_roles};
        let roles: Vec<RbacRole> = rbac_group_roles::table
            .filter(rbac_group_roles::group_id.eq(group_id))
            .inner_join(rbac_roles::table)
            .filter(rbac_roles::is_active.eq(true))
            .select(RbacRole::as_select())
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;
        Ok(serde_json::json!({ "group_id": group_id, "roles": roles }))
    })
    .await;

    match result {
        Ok(Ok(data)) => Json(data).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "get_group_roles", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "get_group_roles", None)).into_response()
        }
    }
}

pub async fn assign_role_to_group(
    State(state): State<Arc<AppState>>,
    Path((group_id, role_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<RbacGroupRole, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_group_roles;
        let assignment = NewRbacGroupRole {
            id: Uuid::new_v4(),
            group_id,
            role_id,
            granted_by: None,
            granted_at: Utc::now(),
        };
        diesel::insert_into(rbac_group_roles::table)
            .values(&assignment)
            .get_result::<RbacGroupRole>(&mut db_conn)
            .map_err(|e| format!("Insert error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(gr)) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "rbac",
                actor_id: Uuid::nil(),
                action: "group.assign_role",
                target_type: Some("group"),
                target_id: Some(group_id),
                success: true,
                details: Some(&format!("Assigned role {role_id} to group {group_id}")),
            });
            (StatusCode::CREATED, Json(gr)).into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "assign_role_to_group", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "assign_role_to_group", None)).into_response()
        }
    }
}

pub async fn remove_role_from_group(
    State(state): State<Arc<AppState>>,
    Path((group_id, role_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_group_roles;
        diesel::delete(
            rbac_group_roles::table
                .filter(rbac_group_roles::group_id.eq(group_id))
                .filter(rbac_group_roles::role_id.eq(role_id)),
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
                action: "group.revoke_role",
                target_type: Some("group"),
                target_id: Some(group_id),
                success: true,
                details: Some(&format!("Revoked role {role_id} from group {group_id}")),
            });
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "remove_role_from_group", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "remove_role_from_group", None)).into_response()
        }
    }
}
