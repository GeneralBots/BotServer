use super::*;

pub async fn catalog_handler(headers: axum::http::HeaderMap) -> Json<serde_json::Value> {
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    let apps = registry::all_apps();

    let (enabled, preview): (
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
    ) = botcore::product::PRODUCT_CONFIG
        .read()
        .map(|c| {
            (
                normalize_email_app_alias(c.get_enabled_apps().into_iter().collect()),
                normalize_email_app_alias(c.get_preview_apps().into_iter().collect()),
            )
        })
        .unwrap_or_default();

    let derived: Vec<crate::core::bot::commands_derived::DerivedCommand> =
        crate::core::bot::commands_derived::derived_commands();

    // #1386c — Vibe-first launcher: the catalog carries EVERY app, each one
    // flagged with `launcher_default`. The launcher surfaces (start menu,
    // sidebar apps rail) show only the flagged ones (Vibe, plus the dynamic
    // bot/published-app tiles) because every other native surface is a
    // toolwindow pane of the Vibe workbench instead of a desktop app — but
    // they must stay in the catalog, since the workbench opens them
    // programmatically (`Vibe → Chat/Terminal/Browser/Database`) and the
    // command palette and agent-control resolve them by id. Dynamic tiles
    // follow below unchanged.
    let items: Vec<serde_json::Value> = apps
        .iter()
        .map(|a| {
            let id = a.id.as_str();
            // #1409 — permanently-visible apps are compiled and enabled no
            // matter what the product file (or feature map) says.
            let always = ALWAYS_APPS.contains(&id);
            let compiled = always || is_app_compiled(id) || CORE_APPS.contains(&id);
            let is_enabled = always || enabled.contains(id) || CORE_APPS.contains(&id);
            // A preview application is listed by `.product`'s `preview_apps`
            // instead of `apps`, so the launcher can withhold it until the user
            // turns Preview mode on (#1348). Permanently-visible apps are never
            // preview.
            let is_preview = !always && preview.contains(id) && !is_enabled;
            // Merge curated commands with the harvested on-the-fly surface.
            let app_derived: Vec<serde_json::Value> = derived
                .iter()
                .filter(|c| c.name.starts_with(&format!("{id}.")))
                .take(40)
                .map(|c| {
                    json!({
                        "name": c.name,
                        "method": c.method,
                        "path": c.path,
                        "summary": c.summary,
                        "derived": true,
                    })
                })
                .collect();
            let mut commands: Vec<serde_json::Value> = commands::commands_for_app(id)
                .iter()
                .map(|c| {
                    json!({ "name": c.name, "label": c.label, "summary": c.summary, "derived": false })
                })
                .collect();
            commands.extend(app_derived);
            json!({
                "id": a.id,
                "title": a.title,
                "category": a.category,
                "color": a.color,
                "url": a.url,
                "description": a.description,
                "keywords": a.keywords,
                "icon": a.icon,
                "kind": a.kind,
                "widget": a.widget,
                // #1409 — permanently-visible apps are pinned for the
                // launcher too, so they show in the start menu and the
                // sidebar rail even though they are not in `.product`'s
                // `apps=` list.
                "launcher_default": always || a.launcher_default,
                "enabled": is_enabled,
                "preview": is_preview,
                "compiled": compiled,
                "commands": commands,
                "deep_link_params": commands::deep_link_params_for_app(id),
            })
        })
        .collect();

    let _derived_count = derived.len();
    // #1289/#1291 — append user bots after the static registry so they always
    // surface in the launcher (frontend merges by id; static wins nothing
    // since bot ids are namespaced `bot-`).
    let mut all_items = items;
    all_items.extend(bot_launcher_apps(bearer.as_deref()));
    all_items.extend(published_app_launcher_apps(bearer.as_deref()));
    Json(json!({
        "apps": all_items,
        "categories": registry::CATEGORIES
            .iter()
            .map(|(k, l)| json!({"id": k, "label": l}))
            .collect::<Vec<_>>(),
        "labels": registry::category_labels(),
        "surface": json!({ "curated_commands": commands::all_commands().len(), "harvested_endpoints": _derived_count }),
    }))
}
