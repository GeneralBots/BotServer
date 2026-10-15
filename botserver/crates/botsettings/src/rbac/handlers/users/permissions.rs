use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::models::{
    RbacGroup, RbacPermission, RbacRole, User,
};
use botcore::shared::state::AppState;
use botsecurity::auth::AuthenticatedUser;
use botsecurity::error_sanitizer::log_and_sanitize_str;

use crate::rbac::{CheckPermissionRequest, CheckPermissionResponse, PaginationParams};
use crate::rbac::utils;

pub async fn list_users_with_roles(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let search = params.search.clone();

    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{rbac_user_roles, rbac_roles, users};

        let offset_val = (page - 1) * per_page;
        let pattern = search.as_ref().map(|term| format!("%{}%", term.to_lowercase()));

        let users_list: Vec<User> = if let Some(ref pat) = pattern {
            users::table
                .filter(users::is_active.eq(true))
                .filter(
                    users::username.ilike(pat)
                        .or(users::email.ilike(pat)),
                )
                .order(users::username.asc())
                .offset(offset_val)
                .limit(per_page)
                .load(&mut db_conn)
                .map_err(|e| format!("Query error: {e}"))?
        } else {
            users::table
                .filter(users::is_active.eq(true))
                .order(users::username.asc())
                .offset(offset_val)
                .limit(per_page)
                .load(&mut db_conn)
                .map_err(|e| format!("Query error: {e}"))?
        };

        let users_with_roles: Vec<serde_json::Value> = users_list
            .into_iter()
            .map(|u| {
                let user_roles: Vec<RbacRole> = rbac_user_roles::table
                    .inner_join(rbac_roles::table)
                    .filter(rbac_user_roles::user_id.eq(u.id))
                    .filter(rbac_roles::is_active.eq(true))
                    .select(RbacRole::as_select())
                    .load(&mut db_conn)
                    .unwrap_or_default();
                serde_json::json!({
                    "id": u.id,
                    "email": u.email,
                    "username": u.username,
                    "roles": user_roles,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "users": users_with_roles,
            "page": page,
            "per_page": per_page,
        }))
    })
    .await;

    match result {
        Ok(Ok(data)) => Json(data).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "list_users_with_roles", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "list_users_with_roles", None)).into_response()
        }
    }
}

pub async fn get_effective_permissions(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<RbacPermission>, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{
            rbac_group_roles, rbac_permissions, rbac_role_permissions, rbac_user_groups,
            rbac_user_roles,
        };

        let direct_role_ids: Vec<Uuid> = rbac_user_roles::table
            .filter(rbac_user_roles::user_id.eq(user_id))
            .select(rbac_user_roles::role_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let user_group_ids: Vec<Uuid> = rbac_user_groups::table
            .filter(rbac_user_groups::user_id.eq(user_id))
            .select(rbac_user_groups::group_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let mut all_group_ids: Vec<Uuid> = vec![];
        let mut visited: Vec<Uuid> = vec![];
        for gid in &user_group_ids {
            let expanded = utils::resolve_group_ids(*gid, &mut db_conn, &mut visited)?;
            all_group_ids.extend(expanded);
        }
        all_group_ids.sort();
        all_group_ids.dedup();

        let group_role_ids: Vec<Uuid> = rbac_group_roles::table
            .filter(rbac_group_roles::group_id.eq_any(&all_group_ids))
            .select(rbac_group_roles::role_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let mut all_role_ids: Vec<Uuid> = direct_role_ids;
        all_role_ids.extend(group_role_ids);
        all_role_ids.sort();
        all_role_ids.dedup();

        let permission_ids: Vec<Uuid> = rbac_role_permissions::table
            .filter(rbac_role_permissions::role_id.eq_any(&all_role_ids))
            .select(rbac_role_permissions::permission_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let permissions: Vec<RbacPermission> = rbac_permissions::table
            .filter(rbac_permissions::id.eq_any(&permission_ids))
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        Ok(permissions)
    })
    .await;

    match result {
        Ok(Ok(permissions)) => Json(serde_json::json!({
            "user_id": user_id,
            "permissions": permissions,
        })).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "get_effective_permissions", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "get_effective_permissions", None)).into_response()
        }
    }
}

pub async fn check_permission(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
    Json(req): Json<CheckPermissionRequest>,
) -> impl IntoResponse {
    let permission_str = req.permission.to_lowercase();

    if user.is_admin() || user.is_super_admin() {
        return Json(CheckPermissionResponse {
            granted: true,
            permission: permission_str,
            source: "admin_bypass".to_string(),
        })
        .into_response();
    }

    let conn = state.conn.clone();
    let user_id = user.user_id;
    let perm_str = permission_str.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<bool, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        utils::user_has_db_permission(user_id, &perm_str, &mut db_conn)
    })
    .await;

    match result {
        Ok(Ok(granted)) => Json(CheckPermissionResponse {
            granted,
            permission: permission_str,
            source: if granted { "db_role_permission" } else { "denied" }.to_string(),
        })
        .into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "check_permission", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "check_permission", None)).into_response()
        }
    }
}

pub async fn my_permissions(
    State(state): State<Arc<AppState>>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let user_id = user.user_id;
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::models::schema::{
            rbac_group_roles, rbac_groups, rbac_permissions, rbac_role_permissions, rbac_roles,
            rbac_user_groups, rbac_user_roles,
        };

        let direct_roles: Vec<RbacRole> = rbac_user_roles::table
            .inner_join(rbac_roles::table)
            .filter(rbac_user_roles::user_id.eq(user_id))
            .filter(rbac_roles::is_active.eq(true))
            .select(RbacRole::as_select())
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let user_group_ids: Vec<Uuid> = rbac_user_groups::table
            .filter(rbac_user_groups::user_id.eq(user_id))
            .select(rbac_user_groups::group_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let mut all_group_ids: Vec<Uuid> = vec![];
        let mut visited: Vec<Uuid> = vec![];
        for gid in &user_group_ids {
            let expanded = utils::resolve_group_ids(*gid, &mut db_conn, &mut visited)?;
            all_group_ids.extend(expanded);
        }
        all_group_ids.sort();
        all_group_ids.dedup();

        let groups: Vec<RbacGroup> = rbac_user_groups::table
            .inner_join(rbac_groups::table)
            .filter(rbac_user_groups::user_id.eq(user_id))
            .filter(rbac_groups::is_active.eq(true))
            .select(RbacGroup::as_select())
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let group_roles: Vec<RbacRole> = rbac_group_roles::table
            .inner_join(rbac_roles::table)
            .filter(rbac_group_roles::group_id.eq_any(&all_group_ids))
            .filter(rbac_roles::is_active.eq(true))
            .select(RbacRole::as_select())
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let mut all_role_ids: Vec<Uuid> = Vec::new();
        for r in &direct_roles {
            all_role_ids.push(r.id);
        }
        for r in &group_roles {
            all_role_ids.push(r.id);
        }
        all_role_ids.sort();
        all_role_ids.dedup();

        let permission_ids: Vec<Uuid> = rbac_role_permissions::table
            .filter(rbac_role_permissions::role_id.eq_any(&all_role_ids))
            .select(rbac_role_permissions::permission_id)
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        let permissions: Vec<RbacPermission> = rbac_permissions::table
            .filter(rbac_permissions::id.eq_any(&permission_ids))
            .load(&mut db_conn)
            .map_err(|e| format!("Query error: {e}"))?;

        Ok::<_, String>(serde_json::json!({
            "user_id": user_id,
            "direct_roles": direct_roles,
            "group_roles": group_roles,
            "groups": groups,
            "permissions": permissions,
        }))
    })
    .await;

    match result {
        Ok(Ok(data)) => Json(data).into_response(),
        Ok(Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e, "my_permissions", None)).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, log_and_sanitize_str(&e.to_string(), "my_permissions", None)).into_response()
        }
    }
}
