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
    NewRbacRole, NewRbacRolePermission, RbacPermission, RbacRole,
};
use botcore::shared::state::AppState;
use botsecurity::error_sanitizer::log_and_sanitize_str;
use diesel::prelude::*;

use super::super::{CreateRoleRequest, UpdateRolePermissionsRequest};

pub async fn list_roles(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<RbacRole>, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_roles;
        rbac_roles::table
            .filter(rbac_roles::is_active.eq(true))
            .order(rbac_roles::display_name.asc())
            .load::<RbacRole>(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(roles)) => Json(serde_json::json!({ "roles": roles })).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "list_roles", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "list_roles", None)).into_response()
        }
    }
}

pub async fn get_role(State(state): State<Arc<AppState>>, Path(role_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<RbacRole, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_roles;
        rbac_roles::table
            .find(role_id)
            .first::<RbacRole>(&mut db_conn)
            .map_err(|e| format!("Role not found: {e}"))
    })
    .await;

    match result {
        Ok(Ok(role)) => Json(role).into_response(),
        Ok(Err(e)) => {
            (StatusCode::NOT_FOUND, log_and_sanitize_str(&e, "get_role", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "get_role", None)).into_response()
        }
    }
}

pub async fn create_role(State(state): State<Arc<AppState>>, Json(req): Json<CreateRoleRequest>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let now = Utc::now();
    let new_role = NewRbacRole {
        id: Uuid::new_v4(),
        name: req.name.to_lowercase().replace(' ', "_"),
        display_name: req.display_name,
        description: req.description,
        is_system: false,
        is_active: true,
        created_by: None,
        created_at: now,
        updated_at: now,
    };

    let result = tokio::task::spawn_blocking(move || -> Result<RbacRole, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_roles;
        diesel::insert_into(rbac_roles::table)
            .values(&new_role)
            .get_result::<RbacRole>(&mut db_conn)
            .map_err(|e| format!("Insert error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(role)) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "rbac",
                actor_id: Uuid::nil(),
                action: "role.create",
                target_type: Some("role"),
                target_id: Some(role.id),
                success: true,
                details: Some(&format!("Created role '{}'", role.name)),
            });
            (StatusCode::CREATED, Json(role)).into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "create_role", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "create_role", None)).into_response()
        }
    }
}

pub async fn update_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<Uuid>,
    Json(req): Json<CreateRoleRequest>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let now = Utc::now();
    let result = tokio::task::spawn_blocking(move || -> Result<RbacRole, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_roles;
        let role: RbacRole = rbac_roles::table
            .find(role_id)
            .first(&mut db_conn)
            .map_err(|e| format!("Role not found: {e}"))?;
        if role.is_system {
            return Err("Cannot modify system role".to_string());
        }
        diesel::update(rbac_roles::table.find(role_id))
            .set((
                rbac_roles::display_name.eq(&req.display_name),
                rbac_roles::description.eq(&req.description),
                rbac_roles::name.eq(req.name.to_lowercase().replace(' ', "_")),
                rbac_roles::updated_at.eq(now),
            ))
            .get_result::<RbacRole>(&mut db_conn)
            .map_err(|e| format!("Update error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(role)) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "rbac",
                actor_id: Uuid::nil(),
                action: "role.update",
                target_type: Some("role"),
                target_id: Some(role.id),
                success: true,
                details: Some(&format!("Updated role '{}'", role.name)),
            });
            Json(role).into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "update_role", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "update_role", None)).into_response()
        }
    }
}

pub async fn delete_role(State(state): State<Arc<AppState>>, Path(role_id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_roles;
        diesel::update(rbac_roles::table.find(role_id))
            .set(rbac_roles::is_active.eq(false))
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
                action: "role.delete",
                target_type: Some("role"),
                target_id: Some(role_id),
                success: true,
                details: Some("Deactivated role"),
            });
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "delete_role", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "delete_role", None)).into_response()
        }
    }
}

pub async fn list_permissions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<RbacPermission>, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::rbac_permissions;
        rbac_permissions::table
            .order(rbac_permissions::category.asc())
            .then_order_by(rbac_permissions::display_name.asc())
            .load::<RbacPermission>(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(permissions)) => Json(serde_json::json!({ "permissions": permissions })).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "list_permissions", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "list_permissions", None)).into_response()
        }
    }
}

pub async fn get_role_permissions(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<Uuid>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_permissions, rbac_role_permissions};

        let all_permissions = rbac_permissions::table
            .order(rbac_permissions::category.asc())
            .then_order_by(rbac_permissions::display_name.asc())
            .load::<RbacPermission>(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let granted_ids: Vec<Uuid> = rbac_role_permissions::table
            .filter(rbac_role_permissions::role_id.eq(role_id))
            .select(rbac_role_permissions::permission_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let permissions: Vec<serde_json::Value> = all_permissions
            .into_iter()
            .map(|perm| {
                serde_json::json!({
                    "id": perm.id,
                    "name": perm.name,
                    "display_name": perm.display_name,
                    "description": perm.description,
                    "resource_type": perm.resource_type,
                    "action": perm.action,
                    "category": perm.category,
                    "granted": granted_ids.contains(&perm.id),
                })
            })
            .collect();

        Ok(serde_json::json!({ "role_id": role_id, "permissions": permissions }))
    })
    .await;

    match result {
        Ok(Ok(data)) => Json(data).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "get_role_permissions", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "get_role_permissions", None)).into_response()
        }
    }
}

pub async fn update_role_permissions(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<Uuid>,
    Json(req): Json<UpdateRolePermissionsRequest>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let now = Utc::now();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_role_permissions, rbac_roles};

        let _role: RbacRole = rbac_roles::table
            .find(role_id)
            .first(&mut db_conn)
            .map_err(|e| format!("Role not found: {e}"))?;

        diesel::delete(
            rbac_role_permissions::table.filter(rbac_role_permissions::role_id.eq(role_id)),
        )
        .execute(&mut db_conn)
        .map_err(|e| format!("Delete error: {e}"))?;

        if !req.permission_ids.is_empty() {
            let new_perms: Vec<NewRbacRolePermission> = req
                .permission_ids
                .into_iter()
                .map(|permission_id| NewRbacRolePermission {
                    id: Uuid::new_v4(),
                    role_id,
                    permission_id,
                    granted_by: None,
                    granted_at: now,
                })
                .collect();

            diesel::insert_into(rbac_role_permissions::table)
                .values(&new_perms)
                .execute(&mut db_conn)
                .map_err(|e| format!("Insert error: {e}"))?;
        }

        log::info!("Updated permissions for role {role_id}");
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => StatusCode::OK.into_response(),
        Ok(Err(e)) => {
            (StatusCode::BAD_REQUEST, log_and_sanitize_str(&e, "update_role_permissions", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "update_role_permissions", None)).into_response()
        }
    }
}
