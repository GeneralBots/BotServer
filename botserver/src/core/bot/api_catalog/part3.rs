use super::*;

/// Catalogued-command + endpoint search, filtered by the current user's role.
pub(super) fn find_api_entries(state: &Arc<AppState>, user_id: Uuid, query: &str) -> Vec<Value> {
    let q = tokens_of(query);
    if q.is_empty() {
        return vec![find_hint("the query had no searchable words")];
    }
    let role = crate::security::user_role::resolve_user_role(&state.conn, user_id);
    let mut out: Vec<Value> = Vec::new();
    let mut scored_cmds: Vec<(i32, &AppCommand)> = commands_list()
        .iter()
        .filter_map(|cmd| {
            if cmd.admin_only && role != crate::security::user_role::ROLE_ADMIN {
                return None;
            }
            let hay = format!("{} {}", cmd.name.to_lowercase(), cmd.summary.to_lowercase());
            let score = score_tokens(&hay, &q);
            if score > 0 {
                Some((score, *cmd))
            } else {
                None
            }
        })
        .collect();
    scored_cmds.sort_by_key(|a| std::cmp::Reverse(a.0.clone()));
    for (_, cmd) in scored_cmds.into_iter().take(3) {
        out.push(json!({
            "kind": "command",
            "name": cmd.name,
            "summary": cmd.summary,
            "admin_only": cmd.admin_only,
            "params": cmd.params.iter().map(|(n, d)| json!({"name": n, "description": d})).collect::<Vec<_>>(),
        }));
    }
    // Harvested on-the-fly commands derived from the endpoint inventory. These
    // are proposed as `api.exec` candidates (method + path) for regular users.
    let derived = crate::core::bot::commands_derived::derived_commands();
    let mut scored_dc: Vec<(i32, &crate::core::bot::commands_derived::DerivedCommand)> = derived
        .iter()
        .filter(|c| {
            if role != crate::security::user_role::ROLE_ADMIN
                && crate::security::user_role::is_admin_only_endpoint(&state.conn, user_id, c.method, &c.path)
            {
                return false;
            }
            true
        })
        .filter_map(|c| {
            let hay = format!("{} {}", c.name.to_lowercase(), c.summary.to_lowercase());
            let score = score_tokens(&hay, &q);
            if score > 0 { Some((score, c)) } else { None }
        })
        .collect();
    scored_dc.sort_by_key(|a| std::cmp::Reverse(a.0.clone()));
    for (_, c) in scored_dc.into_iter().take(6) {
        out.push(json!({
            "kind": "derived",
            "name": c.name,
            "summary": c.summary,
            "method": c.method,
            "path": c.path,
            "admin_only": crate::security::user_role::is_admin_only_endpoint(&state.conn, user_id, c.method, &c.path),
        }));
    }
    let mut scored_ep: Vec<(i32, &ApiEndpoint)> = all_endpoints()
        .iter()
        .chain(crate::core::bot::endpoint_inventory::all_routes())
        .filter_map(|ep| {
            if crate::security::user_role::is_admin_only_endpoint(&state.conn, user_id, ep.method, ep.path) {
                return None;
            }
            let hay = format!("{} {} {}", ep.method.to_lowercase(), ep.path.to_lowercase(), ep.summary.to_lowercase());
            let score = score_tokens(&hay, &q);
            if score > 0 {
                Some((score, ep))
            } else {
                None
            }
        })
        .collect();
    // first occurrence wins (curated list precedes the auto inventory)
    scored_ep.sort_by_key(|a| std::cmp::Reverse(a.0.clone()));
    let mut seen = std::collections::HashSet::new();
    for (_, ep) in scored_ep.into_iter().take(6) {
        if !seen.insert((ep.method, ep.path)) {
            continue;
        }
        out.push(json!({
            "kind": "endpoint",
            "method": ep.method,
            "path": ep.path,
            "summary": ep.summary,
        }));
    }
    if out.is_empty() {
        // An empty list is a dead end: the model repeats api.find or gives up.
        // Hand back the vocabulary that actually matches this catalog.
        out.push(find_hint("no command, endpoint or derived action matched the query words"));
    }
    out
}
/// Actionable guidance returned in place of an empty `api.find` result.
pub(super) fn find_hint(reason: &str) -> Value {
    json!({
        "kind": "hint",
        "reason": reason,
        "guidance": "api.find matches command names and endpoint paths by keyword, not user intent. \
                     Retry with a domain word such as 'task', 'automation', 'drive', 'invoice', 'contact' \
                     or 'report', or call a command listed in your instructions directly by name. To turn a \
                     plain-language request into a new automation, call tasks.autotask.create with the request \
                     as the intent parameter.",
    })
}
pub(super) fn normalize_drive_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim().trim_start_matches('/').to_string();
    if trimmed.is_empty() || trimmed.contains("..") {
        return Err("invalid path".to_string());
    }
    Ok(trimmed)
}
