use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use chrono::Utc;
use diesel::RunQueryDsl;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;
use botcoresecrets::manager::SecretsManager;

use crate::settings_api::{get_conn, random_key, resolve_user_id};

// ─────────────────────────── Webhooks ───────────────────────────

pub async fn webhooks_list(
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

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct HookRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        url: String,
        #[diesel(sql_type = diesel::sql_types::Jsonb)]
        events: serde_json::Value,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
    }

    let rows: Vec<HookRow> = diesel::sql_query(
        "SELECT id, url, events, is_active FROM webhooks WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return (
            StatusCode::OK,
            Html(r#"<div class="empty-state"><p>No webhooks configured yet</p></div>"#.to_string()),
        );
    }

    let mut html = String::from("<div class=\"webhook-items\">");
    for hook in &rows {
        let events = hook
            .events
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "*".to_string());
        let badge = if hook.is_active {
            "<span class=\"status-badge active\">Active</span>"
        } else {
            "<span class=\"status-badge revoked\">Disabled</span>"
        };
        html.push_str(&format!(
            r#"<div class="webhook-item" data-webhook-id="{hook_id}">
                <div class="webhook-main">
                    <span class="webhook-url">{url}</span>
                    <span class="webhook-events">Events: {events}</span>
                    {badge}
                </div>
                <div class="webhook-actions">
                    <button class="btn-icon" hx-post="/api/user/webhooks/{hook_id}/test" hx-target="closest .webhook-item" hx-swap="outerHTML" title="Send test event">▶</button>
                    <button class="btn-icon" hx-get="/api/user/webhooks/{hook_id}/deliveries" hx-target="closest .webhook-item" hx-swap="outerHTML" title="Delivery log">📋</button>
                    <button class="btn-icon" hx-delete="/api/user/webhooks/{hook_id}" hx-target="closest .webhook-item" hx-swap="outerHTML" title="Remove">×</button>
                </div>
            </div>"#,
            hook_id = hook.id,
            url = crate::webhook_delivery::escape_html(&hook.url),
        ));
    }
    html.push_str("</div>");
    (StatusCode::OK, Html(html))
}

#[derive(serde::Deserialize)]
pub struct WebhookForm {
    pub url: Option<String>,
    pub events: Option<String>,
}

pub async fn webhooks_create(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(form): axum::extract::Form<WebhookForm>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let url = form.url.unwrap_or_default().trim().to_string();
    if let Err(msg) = crate::webhook_delivery::validate_webhook_url(&url) {
        return (StatusCode::BAD_REQUEST, Html(msg.to_string()));
    }

    let webhook_id = Uuid::new_v4();
    let events = crate::webhook_delivery::parse_events(form.events.as_deref());

    // Generate the signing secret and store it in Vault — never in the DB.
    let secret = format!("whsec_{}", random_key(40));
    let vault_path = crate::webhook_delivery::webhook_vault_path(webhook_id);
    let mut vault_data = std::collections::HashMap::new();
    vault_data.insert("secret".to_string(), secret.clone());
    if let Ok(sm) = SecretsManager::get() {
        if let Err(e) = sm.put_secret(&vault_path, vault_data).await {
            log::warn!("webhooks_create: Vault write failed for {vault_path}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to store webhook secret".to_string()));
        }
    } else {
        log::warn!("webhooks_create: SecretsManager unavailable; storing vault path only");
    }

    let inserted = diesel::sql_query(
        "INSERT INTO webhooks (id, user_id, url, events, is_active, secret_vault_path, created_at, updated_at) \
         VALUES ($1, $2, $3, $4::jsonb, true, $5, NOW(), NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(webhook_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(&url)
    .bind::<diesel::sql_types::Text, _>(&events.to_string())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(Some(vault_path))
    .execute(&mut conn);

    if let Err(e) = inserted {
        log::error!("webhooks_create insert failed: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to register webhook".to_string()));
    }

    crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
        state: &state,
        event_type: "webhook",
        actor_id: user_id,
        action: "webhook.create",
        target_type: Some("webhook"),
        target_id: Some(webhook_id),
        success: true,
        details: Some(&format!("registered webhook for {url}")),
    });

    (
        StatusCode::OK,
        Html(format!(
            r#"<div class="webhook-created">
                <p>Webhook registered. Signing secret (shown once):</p>
                <code class="api-key-secret">{secret}</code>
                <p class="hint">Deliveries are signed with <code>X-Webhook-Signature: v1=&lt;hmac&gt;</code>.</p>
            </div>"#
        )),
    )
}

/// DELETE /api/user/webhooks/{webhook_id}
pub async fn webhooks_delete(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(webhook_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let deleted = diesel::sql_query(
        "DELETE FROM webhooks WHERE id = $1 AND user_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(webhook_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .execute(&mut conn);

    match deleted {
        Ok(1) => {
            // Best-effort cleanup of the Vault secret.
            if let Ok(sm) = SecretsManager::get() {
                let _ = sm.delete_secret(&crate::webhook_delivery::webhook_vault_path(webhook_id)).await;
            }
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "webhook",
                actor_id: user_id,
                action: "webhook.delete",
                target_type: Some("webhook"),
                target_id: Some(webhook_id),
                success: true,
                details: None,
            });
            (StatusCode::OK, Html(String::new()))
        }
        Ok(_) => (StatusCode::NOT_FOUND, Html("Webhook not found".to_string())),
        Err(e) => {
            log::error!("webhooks_delete failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to delete webhook".to_string()))
        }
    }
}

/// POST /api/user/webhooks/{webhook_id}/test — sends a signed test event and
/// records the delivery in the log.
pub async fn webhooks_test(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(webhook_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct HookRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        url: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        secret_vault_path: Option<String>,
    }

    let row: Option<HookRow> = diesel::sql_query(
        "SELECT url, secret_vault_path FROM webhooks WHERE id = $1 AND user_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(webhook_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .get_result(&mut conn)
    .ok();

    let Some(row) = row else {
        return (StatusCode::NOT_FOUND, Html("Webhook not found".to_string()));
    };

    // Load the signing secret from Vault.
    let secret = match row.secret_vault_path.as_deref() {
        Some(path) => match SecretsManager::get() {
            Ok(sm) => sm
                .get_secret(path)
                .await
                .ok()
                .and_then(|data| data.get("secret").cloned())
                .unwrap_or_default(),
            Err(_) => String::new(),
        },
        None => String::new(),
    };

    if secret.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html("Webhook signing secret unavailable".to_string()));
    }

    let payload = serde_json::json!({
        "event": "test",
        "timestamp": Utc::now().to_rfc3339(),
        "data": { "message": "Test event from General Bots" },
    });

    let signature = crate::webhook_delivery::sign_payload(&payload.to_string(), &secret);
    let state_clone = state.clone();
    let delivery_id = Uuid::new_v4();
    let url = row.url.clone();

    // Record a pending delivery row, then deliver in the background so the
    // request returns promptly; failures retry with backoff and land in the log.
    let _ = diesel::sql_query(
        "INSERT INTO webhook_deliveries (id, webhook_id, event, payload, signature, status, attempt, max_attempts, created_at) \
         VALUES ($1, $2, 'test', $3::jsonb, $4, 'pending', 0, 5, NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(delivery_id)
    .bind::<diesel::sql_types::Uuid, _>(webhook_id)
    .bind::<diesel::sql_types::Text, _>(&payload.to_string())
    .bind::<diesel::sql_types::Text, _>(&signature)
    .execute(&mut conn);

    let url_clone = url.clone();
    let payload_str = payload.to_string();
    let payload_clone = payload_str.clone();
    let secret_clone = secret.clone();
    tokio::spawn(async move {
        crate::webhook_delivery::deliver_with_retries(
            state_clone,
            delivery_id,
            url_clone,
            &payload_clone,
            &secret_clone,
        )
        .await;
    });

    (
        StatusCode::OK,
        Html(format!(
            r#"<div class="webhook-created"><p>Test event sent to <code>{url}</code> — see the delivery log.</p></div>"#
        )),
    )
}

/// GET /api/user/webhooks/{webhook_id}/deliveries — delivery history.
pub async fn webhook_deliveries_list(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(webhook_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html(String::new())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Ownership check on the webhook.
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct OwnedRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        owned: i64,
    }
    let owned: OwnedRow = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS owned FROM webhooks WHERE id = $1 AND user_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(webhook_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .get_result(&mut conn)
    .unwrap_or(OwnedRow { owned: 0 });

    if owned.owned == 0 {
        return (StatusCode::NOT_FOUND, Html("Webhook not found".to_string()));
    }

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct DeliveryRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        event: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        status: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
        response_code: Option<i32>,
        #[diesel(sql_type = diesel::sql_types::Int4)]
        attempt: i32,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
        completed_at: Option<chrono::DateTime<Utc>>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        error: Option<String>,
    }

    let rows: Vec<DeliveryRow> = diesel::sql_query(
        "SELECT event, status, response_code, attempt, completed_at, error \
         FROM webhook_deliveries WHERE webhook_id = $1 ORDER BY created_at DESC LIMIT 20",
    )
    .bind::<diesel::sql_types::Uuid, _>(webhook_id)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return (
            StatusCode::OK,
            Html(r#"<div class="empty-state"><p>No deliveries yet</p></div>"#.to_string()),
        );
    }

    let mut html = String::from("<div class=\"webhook-deliveries\"><table class=\"invoices-table\"><thead><tr><th>Event</th><th>Status</th><th>Code</th><th>Attempt</th><th>Completed</th></tr></thead><tbody>");
    for d in &rows {
        let code = d
            .response_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "—".to_string());
        let completed = d
            .completed_at
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "—".to_string());
        let err = d.error.as_deref().unwrap_or("");
        html.push_str(&format!(
            r#"<tr><td>{}</td><td><span class="status-badge {}">{}</span></td><td>{code}</td><td>{}</td><td>{completed}</td></tr>"#,
            crate::webhook_delivery::escape_html(&d.event),
            match d.status.as_str() {
                "success" => "paid",
                "pending" => "active",
                _ => "revoked",
            },
            crate::webhook_delivery::escape_html(&d.status),
            d.attempt,
        ));
        if !err.is_empty() {
            html.push_str(&format!(
                r#"<tr><td colspan="5" class="hint">{}</td></tr>"#,
                crate::webhook_delivery::escape_html(err)
            ));
        }
    }
    html.push_str("</tbody></table></div>");
    (StatusCode::OK, Html(html))
}
