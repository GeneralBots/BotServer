use super::*;

pub(super) fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub(super) fn on_or_true(v: &Option<String>) -> bool {
    v.as_deref().map(|s| s == "on" || s == "true").unwrap_or(false)
}
pub async fn notif_prefs_get(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html(String::new())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let prefs = read_pref(&mut conn, user_id, "notifications");
    let email_dm = prefs.get("email_dm").and_then(|v| v.as_bool()).unwrap_or(true);
    let email_mentions = prefs.get("email_mentions").and_then(|v| v.as_bool()).unwrap_or(true);
    let email_digest = prefs.get("email_digest").and_then(|v| v.as_bool()).unwrap_or(false);
    let email_marketing = prefs.get("email_marketing").and_then(|v| v.as_bool()).unwrap_or(false);
    let push_enabled = prefs.get("push_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let push_sound = prefs.get("push_sound").and_then(|v| v.as_bool()).unwrap_or(true);
    let desktop_notifications = prefs.get("desktop_notifications").and_then(|v| v.as_bool()).unwrap_or(true);
    let badge_count = prefs.get("badge_count").and_then(|v| v.as_bool()).unwrap_or(true);

    (StatusCode::OK, Html(format!(
        r#"<div class="preference-list">
    <label class="checkbox-label"><input type="checkbox" name="email_dm" {email_dm_ck}> Direct messages</label>
    <label class="checkbox-label"><input type="checkbox" name="email_mentions" {email_mentions_ck}> Mentions</label>
    <label class="checkbox-label"><input type="checkbox" name="email_digest" {email_digest_ck}> Weekly digest</label>
    <label class="checkbox-label"><input type="checkbox" name="email_marketing" {email_marketing_ck}> Marketing</label>
    <label class="checkbox-label"><input type="checkbox" name="push_enabled" {push_enabled_ck}> Push notifications</label>
    <label class="checkbox-label"><input type="checkbox" name="push_sound" {push_sound_ck}> Sound</label>
    <label class="checkbox-label"><input type="checkbox" name="desktop_notifications" {desktop_ck}> Desktop notifications</label>
    <label class="checkbox-label"><input type="checkbox" name="badge_count" {badge_ck}> Badge count</label>
</div>"#,
        email_dm_ck = if email_dm { "checked" } else { "" },
        email_mentions_ck = if email_mentions { "checked" } else { "" },
        email_digest_ck = if email_digest { "checked" } else { "" },
        email_marketing_ck = if email_marketing { "checked" } else { "" },
        push_enabled_ck = if push_enabled { "checked" } else { "" },
        push_sound_ck = if push_sound { "checked" } else { "" },
        desktop_ck = if desktop_notifications { "checked" } else { "" },
        badge_ck = if badge_count { "checked" } else { "" },
    )))
}
pub async fn notif_prefs_put(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(form): axum::extract::Form<NotifPrefsForm>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let prefs = serde_json::json!({
        "email_dm": on_or_true(&form.email_dm),
        "email_mentions": on_or_true(&form.email_mentions),
        "email_digest": on_or_true(&form.email_digest),
        "email_marketing": on_or_true(&form.email_marketing),
        "push_enabled": on_or_true(&form.push_enabled),
        "push_sound": on_or_true(&form.push_sound),
        "desktop_notifications": on_or_true(&form.desktop_notifications),
        "badge_count": on_or_true(&form.badge_count),
    });
    write_pref(&mut conn, user_id, "notifications", &prefs);

    (StatusCode::OK, Html("Notification preferences saved".to_string()))
}
/// POST /api/oauth/{provider}/connect — starts the account-linking flow.
/// Delegates to [`crate::settings_oauth::oauth_connect`], which builds a
/// user-bound OAuth state (PKCE) and returns the provider redirect URL.
pub(super) async fn oauth_connect_handler(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(provider_name): axum::extract::Path<String>,
) -> axum::response::Response {
    crate::settings_oauth::oauth_connect(
        State(state),
        headers,
        axum::extract::Path(provider_name),
    )
    .await
}
