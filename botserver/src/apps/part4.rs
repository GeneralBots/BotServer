use super::*;

/// #1289/#1291 — user bots as first-class launcher apps. Every active bot
/// becomes a chat tile (`bot-{slug}`) that opens the Chat window bound to
/// THAT bot via the `bot` deep-link param (chat-init consumes
/// `__gbAppParams__.bot` and calls ChatSwitchBot). Query is best-effort: a
/// DB hiccup degrades to the static catalog only.
pub(super) fn bot_launcher_apps(bearer: Option<&str>) -> Vec<serde_json::Value> {
    use diesel::prelude::*;
    let pool = match botcore::shared::utils::create_conn() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("apps catalog: bots listing skipped (db: {e})");
            return Vec::new();
        }
    };
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct BotRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        description: Option<String>,
    }
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("apps catalog: bots listing skipped (pool: {e})");
            return Vec::new();
        }
    };
    // #1296 — the launcher must list only the caller's organization bots,
    // not every active bot across all tenants.
    // #1386 — scope resolution now uses the canonical tenant resolver
    // (botsecurity_core::tenant) instead of hand-rolled claim parsing, so
    // email-bound sessions resolve the same branch everywhere else does.
    // #1320 — scope is org AND branch: without the branch predicate a user
    // belonging to one workspace saw every bot of the org's other branches.
    // Anonymous/unbound callers get the single global default bot ONLY —
    // the previous `is_default_for_branch = true` fallback listed 30 bots
    // from every org in production (cross-tenant leak).
    let headers = bearer
        .map(|b| {
            let mut h = axum::http::HeaderMap::new();
            h.insert(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_str(&format!("Bearer {b}"))
                    .unwrap_or(axum::http::HeaderValue::from_static("")),
            );
            h
        })
        .unwrap_or_default();
    let org_filter = botsecurity_core::tenant::org_from_claims(&headers);
    let branch_filter = botsecurity_core::tenant::branch_from_claims(&headers);
    // #1386 follow-up — vibe-origin bots (`bots.origin = 'vibe'`, bot-kind
    // Vibe projects) are TEST bots: they are reachable through the Vibe
    // workbench (Run window / Chat button) and /chat/{slug}, never as
    // launcher tiles. Only drive-origin (production) bots are listed.
    let rows: Vec<BotRow> = match (org_filter, branch_filter) {
        (Some(org_id), Some(branch_id)) => diesel::sql_query(
            "SELECT name, description FROM bots \
             WHERE is_active = true AND org_id = $1 AND branch_id = $2 \
             AND (origin = 'drive' OR origin IS NULL) \
             ORDER BY created_at DESC LIMIT 40",
        )
        .bind::<diesel::sql_types::Uuid, _>(org_id)
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .load(&mut conn)
        .unwrap_or_default(),
        (Some(org_id), None) => {
            // Branch-less tokens see only this org's DEFAULT bots — every
            // other org's bots stay invisible even when 30 rows worldwide
            // are flagged is_default_for_branch.
            diesel::sql_query(
                "SELECT name, description FROM bots \
                 WHERE is_active = true AND org_id = $1 AND is_default_for_branch = true \
                 AND (origin = 'drive' OR origin IS NULL) \
                 ORDER BY created_at DESC LIMIT 40",
            )
            .bind::<diesel::sql_types::Uuid, _>(org_id)
            .load(&mut conn)
            .unwrap_or_default()
        }
        (None, _) => diesel::sql_query(
            "SELECT name, description FROM bots \
             WHERE is_active = true AND id = 'f47ac10b-58cc-4372-a567-0e02b2c3d480'::uuid \
             ORDER BY created_at DESC LIMIT 40",
        )
        .load(&mut conn)
        .unwrap_or_default(),
    };
    rows.into_iter()
        .map(|b| {
            // Slug must be URL/launcher-safe: names like "Helper Bot" would
            // otherwise produce ids with spaces and deep-links that never
            // match the bot's chat route.
            let slug = b
                .name
                .to_lowercase()
                .replace(' ', "-")
                .replace('_', "-")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect::<String>();
            json!({
                "id": format!("bot-{slug}"),
                "title": b.name,
                "category": "ai",
                "color": "#84d669",
                "url": "/suite/partials/chat.html",
                "description": b.description.unwrap_or_else(|| format!("Chat with the {} assistant.", b.name)),
                "keywords": format!("bot chat assistant {slug}"),
                "icon": "<rect x=\"4\" y=\"8\" width=\"16\" height=\"12\" rx=\"2\"/><path d=\"M12 8V4M8 4h8\"/><circle cx=\"9\" cy=\"13\" r=\"1\"/><circle cx=\"15\" cy=\"13\" r=\"1\"/>",
                "kind": "app",
                "widget": serde_json::Value::Null,
                "launcher_default": true,
                "enabled": true,
                "compiled": true,
                "commands": [],
                "deep_link_params": { "bot": slug },
            })
        })
        .collect()
}
/// #1291 — vibe-published apps as launcher tiles. Every `apps`/`website`
/// project whose publish recorded a launcher entry (`payload.launcher.enabled`)
/// surfaces with id `vibeapp-{slug}`; the tile opens the shared Browser
/// window deep-linked to the app URL (workspace serve route, which proxies
/// through the platform with auth). Query is best-effort: a DB hiccup
/// degrades to the static catalog only.
pub(super) fn published_app_launcher_apps(bearer: Option<&str>) -> Vec<serde_json::Value> {
    use diesel::prelude::*;
    use uuid::Uuid;
    let pool = match botcore::shared::utils::create_conn() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("apps catalog: published apps listing skipped (db: {e})");
            return Vec::new();
        }
    };
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct AppRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        project_type: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        framework: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Jsonb)]
        launcher: serde_json::Value,
    }
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("apps catalog: published apps listing skipped (pool: {e})");
            return Vec::new();
        }
    };
    // #1320 — the launcher must only show apps published by the caller's
    // tenant. Without this filter every org's published sites leaked into
    // every other org's launcher. The org/branch claims come from the same
    // tenant scope the rest of the app uses (bot listing above); an
    // unauthenticated or unbound caller sees only the global (nil) scope.
    // #1386 — claims are read via the canonical tenant resolver so email-
    // bound sessions resolve identically to every other branch-scoped API.
    let headers = bearer
        .map(|b| {
            let mut h = axum::http::HeaderMap::new();
            h.insert(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_str(&format!("Bearer {b}"))
                    .unwrap_or(axum::http::HeaderValue::from_static("")),
            );
            h
        })
        .unwrap_or_default();
    let org_filter = botsecurity_core::tenant::org_from_claims(&headers);
    let branch_filter = botsecurity_core::tenant::branch_from_claims(&headers);
    let nil = Uuid::nil();
    let rows: Vec<AppRow> = diesel::sql_query(
        // The launcher flag is the explicit publish-time opt-in (payload.
        // launcher.enabled); project `status` tracks the VM lifecycle and
        // stays `pending` for proxy-published sites, so it must NOT gate
        // the tile.
        "SELECT id, name, project_type, framework, payload->'launcher' AS launcher \
         FROM vibe_projects \
         WHERE payload->'launcher'->>'enabled' = 'true' \
           AND org_id = $1 AND branch_id = $2 \
         ORDER BY updated_at DESC LIMIT 40",
    )
    .bind::<diesel::sql_types::Uuid, _>(org_filter.unwrap_or(nil))
    .bind::<diesel::sql_types::Uuid, _>(branch_filter.unwrap_or(nil))
    .load(&mut conn)
    .unwrap_or_default();
    rows.into_iter()
        .map(|a| {
            let slug = a
                .name
                .to_lowercase()
                .replace(' ', "-")
                .replace('_', "-")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect::<String>();
            // Browser tile: the workspace serve route renders the app's own
            // index.html through the platform; the Browser app auto-appends
            // the auth token for same-origin /serve/ paths.
            let app_url = format!("/api/vibe/projects/{}/serve/index.html", a.id);
            json!({
                "id": format!("vibeapp-{slug}"),
                "title": a.name,
                "category": "dev",
                "color": "#7c3aed",
                "url": "/suite/browser/browser.html",
                "description": format!(
                    "Vibe-published {} app ({}, {}) — click to open.",
                    if a.project_type == "website" { "website" } else { "runtime" },
                    a.framework.as_deref().unwrap_or("static"),
                    a.launcher
                        .get("kind")
                        .and_then(|k| k.as_str())
                        .unwrap_or("app")
                ),
                "keywords": format!("vibe app published {slug}"),
                "icon": "<rect x=\"3\" y=\"4\" width=\"18\" height=\"14\" rx=\"2\"/><path d=\"M8 2v4M16 2v4M3 10h18\"/>",
                "kind": "app",
                "widget": serde_json::Value::Null,
                "launcher_default": true,
                "enabled": true,
                "compiled": true,
                "commands": [],
                "deep_link_params": { "url": app_url },
            })
        })
        .collect()
}
/// #1409 — "email is the same as mail": any product-file id spelled `email`
/// is normalized to the canonical `mail` app id, so enabling/hiding email in
/// `.product` behaves exactly as if the user wrote `mail`.
pub(super) fn normalize_email_app_alias(mut ids: std::collections::HashSet<String>) -> std::collections::HashSet<String> {
    if ids.remove("email") {
        ids.insert("mail".to_string());
    }
    ids
}
