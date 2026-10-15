use super::*;

pub async fn get_public_bot_config(
    axum::extract::State(state): axum::extract::State<Arc<botcore::shared::state::AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<axum::Json<serde_json::Value>, crate::security::ResponseError> {
    use botcore::shared::models::schema::bots;

    let mut conn = state.conn.get().map_err(|e| {
        log::error!("DB connection error in get_public_bot_config: {}", e);
        crate::security::boxed_error(
            crate::security::error_sanitizer::SafeErrorResponse::internal_error(),
        )
    })?;

    let mut map: HashMap<String, String> = HashMap::new();
    let Some(name) = params.get("bot_name") else {
        return Ok(axum::Json(serde_json::to_value(&map).unwrap_or_default()));
    };

    let bot: Option<(Uuid, bool)> = bots::table
        .filter(bots::name.eq(name))
        .select((bots::id, bots::is_public))
        .first(&mut conn)
        .optional()
        .map_err(|e| {
            log::error!("get_public_bot_config query failed: {e}");
            crate::security::boxed_error(
                crate::security::error_sanitizer::SafeErrorResponse::internal_error(),
            )
        })?;

    let Some((bot_id, is_public)) = bot else {
        return Ok(axum::Json(serde_json::json!({ "is_public": "false" })));
    };

    map.insert("name".to_string(), name.clone());
    map.insert("is_public".to_string(), if is_public { "true" } else { "false" }.to_string());
    if !is_public {
        // Private bot: anonymous callers learn nothing beyond the visibility
        // flag (the chat page uses it to force the login redirect).
        return Ok(axum::Json(serde_json::to_value(&map).unwrap_or_default()));
    }

    let cfg = botcore::config::ConfigManager::new(state.conn.clone());
    let all = cfg.get_all_config(&bot_id);
    for key in [
        "Theme Color",
        "theme_color1",
        "theme-color1",
        "theme_color2",
        "theme-color2",
        "theme_title",
        "theme-title",
        "theme_logo",
        "theme-logo",
        "Website",
        "Answer Mode",
    ] {
        if let Some(value) = all.get(key) {
            map.insert(key.to_string(), value.clone());
        }
    }
    Ok(axum::Json(serde_json::to_value(&map).unwrap_or_default()))
}
