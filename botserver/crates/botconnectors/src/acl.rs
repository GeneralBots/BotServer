use crate::models::ItemRow;
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

/// Principals a user acts as: their own `user:{uuid}` plus every active
/// RBAC group mapped to `group:{name}`. Extra groups (e.g. from JWT claims)
/// are appended.
pub fn principals_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
    extra_groups: Vec<String>,
) -> Result<Vec<String>, String> {
    #[derive(diesel::QueryableByName, Debug)]
    struct GroupNameRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }

    let rows: Vec<GroupNameRow> = diesel::sql_query(
        "SELECT g.name AS name FROM rbac_user_groups ug \
         JOIN rbac_groups g ON g.id = ug.group_id \
         WHERE ug.user_id = $1 AND g.is_active = TRUE",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .load(conn)
    .map_err(|e| format!("Load user groups: {e}"))?;

    let mut principals = vec![format!("user:{user_id}")];
    for row in rows {
        if !principals.contains(&format!("group:{}", row.name)) {
            principals.push(format!("group:{}", row.name));
        }
    }
    for group in extra_groups {
        let principal = format!("group:{group}");
        if !principals.contains(&principal) {
            principals.push(principal);
        }
    }
    Ok(principals)
}

/// Deny-by-default ACL evaluation: an item is visible only when its ACL
/// array is non-empty and shares at least one principal with the caller.
fn acl_allows(acl: &Value, principals: &[String]) -> bool {
    match acl.as_array() {
        Some(entries) => entries
            .iter()
            .filter_map(Value::as_str)
            .any(|principal| principals.contains(&principal.to_string())),
        None => false,
    }
}

/// Permissioned full-text search over indexed items.
///
/// Fetches up to `limit * 4` prefilter rows (SQL handles recency, kind filter
/// and the ILIKE match), then enforces the ACL overlap in code and truncates
/// to `limit`. The post-filter exists because ACL arrays live in JSONB with
/// no relational join target for deny-by-default checks.
pub fn search_visible(
    conn: &mut PgConnection,
    user_id: Uuid,
    q: &str,
    sources: Option<Vec<String>>,
    limit: i64,
) -> Result<Vec<ItemRow>, String> {
    let principals = principals_for_user(conn, user_id, Vec::new())?;

    let effective_limit = limit.clamp(1, 100);
    let prefilter_limit = (effective_limit * 4).max(1);
    let pattern = format!("%{}%", q.trim());

    let kinds = sources.unwrap_or_default();
    let sql = build_search_sql(&kinds);
    let query = diesel::sql_query(sql)
        .bind::<diesel::sql_types::Text, _>(&pattern)
        .bind::<diesel::sql_types::BigInt, _>(prefilter_limit);

    let candidates: Vec<ItemRow> = query
        .load(conn)
        .map_err(|e| format!("Connector search failed: {e}"))?;

    Ok(candidates
        .into_iter()
        .filter(|item| acl_allows(&item.acl, &principals))
        .take(effective_limit as usize)
        .collect())
}

/// SQL with numbered placeholders; `$2..$n+1` carry the optional kind list.
fn build_search_sql(kinds: &[String]) -> String {
    // Kind list is embedded as literals after validation against the closed
    // registry; only the pattern and limit stay bound parameters.
    let valid = ["chat", "mail", "drive"];
    let chosen: Vec<&str> = kinds
        .iter()
        .map(|k| k.as_str())
        .filter(|k| valid.contains(k))
        .collect();
    let kind_filters = if chosen.is_empty() {
        String::new()
    } else {
        let list = chosen
            .iter()
            .map(|k| format!("'{k}'"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" AND c.kind IN ({list})")
    };
    format!(
        "SELECT i.* FROM indexed_items i \
         JOIN connector_connections c ON c.id = i.connection_id \
         WHERE i.deleted_at IS NULL AND c.status = 'connected'{kind_filters} \
           AND (i.title ILIKE $1 OR i.body_tsv ILIKE $1) \
         ORDER BY i.updated_at DESC LIMIT $2"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principals() -> Vec<String> {
        vec![
            "user:00000000-0000-0000-0000-000000000001".to_string(),
            "group:sales".to_string(),
        ]
    }

    #[test]
    fn visible_when_principals_intersect() {
        let acl = serde_json::json!(["group:sales", "group:legal"]);
        assert!(acl_allows(&acl, &principals()));
    }

    #[test]
    fn denies_when_no_overlap() {
        let acl = serde_json::json!(["group:finance"]);
        assert!(!acl_allows(&acl, &principals()));
    }

    #[test]
    fn denies_empty_acl_by_default() {
        assert!(!acl_allows(&serde_json::json!([]), &principals()));
        assert!(!acl_allows(&serde_json::json!(null), &principals()));
        assert!(!acl_allows(&serde_json::Value::String("user:x".to_string()), &principals()));
    }

    #[test]
    fn search_sql_embeds_validated_kind_literals() {
        assert!(build_search_sql(&[]).contains("c.status = 'connected'"));
        assert!(!build_search_sql(&[]).contains("c.kind"));
        let two = build_search_sql(&["chat".to_string(), "mail".to_string()]);
        assert!(two.contains("c.kind IN ('chat', 'mail')"), "got: {two}");
        // Kinds outside the closed registry are ignored, never interpolated.
        let bogus = build_search_sql(&["chat'; DROP TABLE indexed_items --".to_string()]);
        assert!(!bogus.contains("c.kind"), "got: {bogus}");
    }
}
