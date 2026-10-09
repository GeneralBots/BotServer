use super::*;

pub(super) fn active_connection_counts(state: &Arc<AppState>) -> Vec<(String, i64)> {
    let Some(mut conn) = crate::settings_api::get_conn(state) else {
        return Vec::new();
    };

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct ProviderRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        provider_slug: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        total: i64,
    }

    let rows: Vec<ProviderRow> = diesel::sql_query(
        "SELECT provider_slug, COUNT(*)::bigint AS total \
         FROM integration_connections WHERE status = 'active' \
         GROUP BY provider_slug",
    )
    .load::<ProviderRow>(&mut conn)
    .unwrap_or_default();

    rows.into_iter()
        .map(|row| (row.provider_slug, row.total))
        .collect()
}
pub async fn get_accounts_social(State(state): State<Arc<AppState>>) -> Html<String> {
    let connected = active_connection_counts(&state);
    let mut html = String::from("<div class=\"accounts-list\">");
    for &(icon, name, slug) in SOCIAL_PROVIDERS {
        let total = connected
            .iter()
            .find(|(provider, _)| provider == slug)
            .map(|(_, total)| *total)
            .unwrap_or(0);
        let status = if total > 0 {
            format!("<span class=\"account-status connected\">Connected ({total})</span>")
        } else {
            "<span class=\"account-status disconnected\">Not connected</span>".to_string()
        };
        html.push_str(&format!(
            "<div class=\"account-item\"><span class=\"account-icon\">{icon}</span><span class=\"account-name\">{name}</span>{status}</div>"
        ));
    }
    html.push_str("</div>");
    Html(html)
}
pub async fn get_accounts_messaging(State(_state): State<Arc<AppState>>) -> Html<String> {
Html(r##"<div class="accounts-list">
<div class="account-item"><span class="account-icon">💬</span><span class="account-name">Discord</span><span class="account-status disconnected">Not connected</span></div>
<div class="account-item"><span class="account-icon">📱</span><span class="account-name">WhatsApp</span><span class="account-status disconnected">Not connected</span></div>
<div class="account-item"><span class="account-icon">✈️</span><span class="account-name">Telegram</span><span class="account-status disconnected">Not connected</span></div>
<div class="account-item"><span class="account-icon">💼</span><span class="account-name">Teams</span><span class="account-status disconnected">Not connected</span></div>

</div>"##.to_string()) }
pub async fn get_accounts_email(State(state): State<Arc<AppState>>) -> Html<String> {
    let smtp_configured = {
        let Some(mut conn) = crate::settings_api::get_conn(&state) else {
            return Html(String::new());
        };
        let Some(user_id) = crate::settings_api::first_user(&mut conn) else {
            return Html(String::new());
        };
        !crate::settings_api::read_pref(&mut conn, user_id, "smtp_account").is_null()
    };

    let smtp_status = if smtp_configured {
        "<span class=\"account-status connected\">Configured</span>"
    } else {
        "<span class=\"account-status disconnected\">Not configured</span>"
    };

    Html(format!(
        r##"<div class="accounts-list">
<div class="account-item"><span class="account-icon">📧</span><span class="account-name">Gmail</span><span class="account-status disconnected">Not connected</span></div>
<div class="account-item"><span class="account-icon">📨</span><span class="account-name">Outlook</span><span class="account-status disconnected">Not connected</span></div>
<div class="account-item"><span class="account-icon">⚙️</span><span class="account-name">SMTP</span>{smtp_status}</div>

</div>"##
    ))
}
pub async fn save_smtp_account(
State(state): State<Arc<AppState>>,
Json(config): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut conn = match crate::settings_api::get_conn(&state) {
        Some(c) => c,
        None => {
            return Json(serde_json::json!({
                "success": false,
                "error": "Database unavailable"
            }));
        }
    };
    let Some(user_id) = crate::settings_api::first_user(&mut conn) else {
        return Json(serde_json::json!({
            "success": false,
            "error": "No user found"
        }));
    };

    crate::settings_api::write_pref(&mut conn, user_id, "smtp_account", &config);

    Json(serde_json::json!({
        "success": true,
        "message": "SMTP configuration saved",
        "config": config
    }))
}
pub async fn get_storage_info(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut conn = match state.conn.get() {
        Ok(c) => c,
        Err(_) => return Html(String::new()),
    };

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct SizeRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        size: i64,
    }
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct CountRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    let db_size: i64 = diesel::sql_query(
        "SELECT pg_database_size(current_database())::bigint AS size",
    )
    .get_result::<SizeRow>(&mut conn)
    .map(|r| r.size)
    .unwrap_or(0);

    let bot_count: i64 = diesel::sql_query("SELECT COUNT(*)::bigint AS count FROM bots")
        .get_result::<CountRow>(&mut conn)
        .map(|r| r.count)
        .unwrap_or(0);

    let message_count: i64 = diesel::sql_query("SELECT COUNT(*)::bigint AS count FROM message_history")
        .get_result::<CountRow>(&mut conn)
        .map(|r| r.count)
        .unwrap_or(0);

    let source_count: i64 = diesel::sql_query("SELECT COUNT(*)::bigint AS count FROM knowledge_sources")
        .get_result::<CountRow>(&mut conn)
        .map(|r| r.count)
        .unwrap_or(0);

    let used_gb = db_size as f64 / (1024.0 * 1024.0 * 1024.0);

    Html(format!(
        r##"<div class="storage-info">
<div class="storage-details">
<span class="storage-used-text">{used_gb:.2} GB</span>
<span class="storage-total-text">database storage used</span>
</div>
<div class="storage-breakdown">
<div class="storage-item">
<span class="storage-icon">🤖</span>
<span class="storage-label">Bots</span>
<span class="storage-size">{bot_count}</span>
</div>
<div class="storage-item">
<span class="storage-icon">💬</span>
<span class="storage-label">Messages</span>
<span class="storage-size">{message_count}</span>
</div>
<div class="storage-item">
<span class="storage-icon">📄</span>
<span class="storage-label">KB Sources</span>
<span class="storage-size">{source_count}</span>
</div>
</div>
</div>"##
    ))
}
pub async fn get_storage_connections(State(_state): State<Arc<AppState>>) -> Html<String> {
Html(
r##"<div class="connections-empty">
<p class="text-muted">No external storage connections configured</p>
<button class="btn-secondary" onclick="showAddConnectionModal()">
+ Add Connection
</button>

</div>"## .to_string(), ) }
