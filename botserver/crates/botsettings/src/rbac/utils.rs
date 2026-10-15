use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use botcore::shared::models::RbacPermission;

/// Recursively resolve all ancestor group IDs for a given group,
/// traversing the parent_group_id chain upward.
pub fn resolve_group_ids(
    group_id: Uuid,
    db_conn: &mut PgConnection,
    visited: &mut Vec<Uuid>,
) -> Result<Vec<Uuid>, String> {
    if visited.contains(&group_id) {
        return Ok(vec![]);
    }
    visited.push(group_id);

    use botcore::shared::models::schema::rbac_groups;
    let group = rbac_groups::table
        .find(group_id)
        .first::<botcore::shared::models::RbacGroup>(db_conn)
        .map_err(|e| format!("Group query error: {e}"))?;

    let mut result = vec![group_id];

    if let Some(parent_id) = group.parent_group_id {
        let ancestors = resolve_group_ids(parent_id, db_conn, visited)?;
        result.extend(ancestors);
    }

    Ok(result)
}

/// Check if a user has a specific DB-backed permission, resolving
/// through direct roles, group memberships, and group hierarchy.
/// Matches against three formats: PascalCase name, resource_type:action:*,
/// and resource_type:action:name (full colon-delimited alias).
pub fn user_has_db_permission(
    user_id: Uuid,
    permission_str: &str,
    db_conn: &mut PgConnection,
) -> Result<bool, String> {
    use botcore::shared::models::schema::{
        rbac_group_roles, rbac_permissions, rbac_role_permissions, rbac_user_groups,
        rbac_user_roles,
    };

    let direct_role_ids: Vec<Uuid> = rbac_user_roles::table
        .filter(rbac_user_roles::user_id.eq(user_id))
        .select(rbac_user_roles::role_id)
        .load(db_conn)
        .map_err(|e| format!("Query error: {e}"))?;

    let user_group_ids: Vec<Uuid> = rbac_user_groups::table
        .filter(rbac_user_groups::user_id.eq(user_id))
        .select(rbac_user_groups::group_id)
        .load(db_conn)
        .map_err(|e| format!("Query error: {e}"))?;

    let mut all_group_ids: Vec<Uuid> = vec![];
    let mut visited: Vec<Uuid> = vec![];
    for gid in &user_group_ids {
        let expanded = resolve_group_ids(*gid, db_conn, &mut visited)?;
        all_group_ids.extend(expanded);
    }
    all_group_ids.sort();
    all_group_ids.dedup();

    let group_role_ids: Vec<Uuid> = rbac_group_roles::table
        .filter(rbac_group_roles::group_id.eq_any(&all_group_ids))
        .select(rbac_group_roles::role_id)
        .load(db_conn)
        .map_err(|e| format!("Query error: {e}"))?;

    let mut all_role_ids: Vec<Uuid> = direct_role_ids;
    all_role_ids.extend(group_role_ids);
    all_role_ids.sort();
    all_role_ids.dedup();

    if all_role_ids.is_empty() {
        return Ok(false);
    }

    let granted_perm_ids: Vec<Uuid> = rbac_role_permissions::table
        .filter(rbac_role_permissions::role_id.eq_any(&all_role_ids))
        .select(rbac_role_permissions::permission_id)
        .load(db_conn)
        .map_err(|e| format!("Query error: {e}"))?;

    if granted_perm_ids.is_empty() {
        return Ok(false);
    }

    let user_permissions: Vec<RbacPermission> = rbac_permissions::table
        .filter(rbac_permissions::id.eq_any(&granted_perm_ids))
        .load(db_conn)
        .map_err(|e| format!("Query error: {e}"))?;

    let req_lower = permission_str.to_lowercase();

    for perm in &user_permissions {
        let perm_name_lower = perm.name.to_lowercase();

        if botsecurity::rbac_middleware::match_wildcard(&perm_name_lower, &req_lower) {
            return Ok(true);
        }

        let colon_alias = format!("{}:{}:*", perm.resource_type, perm.action);
        if botsecurity::rbac_middleware::match_wildcard(&colon_alias, &req_lower) {
            return Ok(true);
        }

        let colon_alias_full = format!("{}:{}:{}", perm.resource_type, perm.action, perm_name_lower);
        if botsecurity::rbac_middleware::match_wildcard(&colon_alias_full, &req_lower) {
            return Ok(true);
        }
    }

    Ok(false)
}
