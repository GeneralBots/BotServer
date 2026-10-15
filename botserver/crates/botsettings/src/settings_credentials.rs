use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use chrono::Utc;
use diesel::RunQueryDsl;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;

use crate::settings_api::{get_conn, random_key, resolve_user_id};

const KEY_PREFIX: &str = "gb_";

/// Generates a cryptographically-random `gb_` secret and returns it together
/// with its SHA-256 hash. Only the hash is persisted; the raw secret is shown
/// to the caller exactly once.
fn generate_api_key() -> (String, String) {
    let secret = format!("{KEY_PREFIX}{}", random_key(48));
    let hash = hash_secret(&secret);
    (secret, hash)
}

/// SHA-256 hex digest of a key secret (used for both storage and lookup).
pub fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

/// Returns the `gb_` display prefix (first 12 chars) of a secret.
fn display_prefix(secret: &str) -> String {
    let end = secret.len().min(12);
    secret[..end].to_string()
}

fn is_expired(expires_at: Option<chrono::DateTime<Utc>>) -> bool {
    matches!(expires_at, Some(ts) if ts <= Utc::now())
}

fn parse_expiry(days: Option<u64>) -> Option<chrono::DateTime<Utc>> {
    days.map(|d| Utc::now() + chrono::Duration::days(i64::try_from(d).unwrap_or(i64::MAX)))
}

pub async fn api_keys_list(
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
    struct KeyRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        key_prefix: String,
        #[diesel(sql_type = diesel::sql_types::Jsonb)]
        scopes: serde_json::Value,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
        expires_at: Option<chrono::DateTime<Utc>>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
        last_used_at: Option<chrono::DateTime<Utc>>,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
    }

    let rows: Vec<KeyRow> = diesel::sql_query(
        "SELECT id, name, key_prefix, scopes, expires_at, last_used_at, is_active \
         FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return (
            StatusCode::OK,
            Html(r#"<div class="empty-state"><p>No API keys created yet</p></div>"#.to_string()),
        );
    }

    let mut html = String::from("<div class=\"api-key-items\">");
    for key in &rows {
        let scopes = key
            .scopes
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "user".to_string());
        let last_used = key
            .last_used_at
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "never".to_string());
        let expiry = key
            .expires_at
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "never".to_string());
        let state_badge = if !key.is_active {
            "<span class=\"status-badge revoked\">Revoked</span>"
        } else if is_expired(key.expires_at) {
            "<span class=\"status-badge expired\">Expired</span>"
        } else {
            "<span class=\"status-badge active\">Active</span>"
        };
        html.push_str(&format!(
            r#"<div class="api-key-item" data-key-id="{key_id}">
                <div class="api-key-main">
                    <span class="api-key-name">{name}</span>
                    <code class="api-key-value">{prefix}</code>
                    <span class="api-key-meta">Scopes: {scopes} · Last used: {last_used} · Expires: {expiry}</span>
                    {state_badge}
                </div>
                <div class="api-key-actions">
                    <button class="btn-icon" hx-post="/api/user/api-keys/{key_id}/rotate" hx-target="closest .api-key-item" hx-swap="outerHTML" title="Rotate">⟳</button>
                    <button class="btn-icon" hx-delete="/api/user/api-keys/{key_id}" hx-target="closest .api-key-item" hx-swap="outerHTML" title="Revoke">×</button>
                </div>
            </div>"#,
            key_id = key.id,
            name = crate::webhook_delivery::escape_html(&key.name),
            prefix = crate::webhook_delivery::escape_html(&key.key_prefix),
        ));
    }
    html.push_str("</div>");
    (StatusCode::OK, Html(html))
}

#[derive(serde::Deserialize)]
pub struct ApiKeyForm {
    pub name: Option<String>,
    pub scopes: Option<String>,
    pub expires_in_days: Option<u64>,
}

pub async fn api_keys_create(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(form): axum::extract::Form<ApiKeyForm>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let name = form.name.unwrap_or_else(|| "API Key".to_string());
    if name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Html("API key name is required".to_string()));
    }

    let (secret, hash) = generate_api_key();
    let scopes = parse_scopes(form.scopes.as_deref());

    let inserted = diesel::sql_query(
        "INSERT INTO api_keys (id, user_id, name, key_hash, key_prefix, scopes, expires_at, is_active, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, true, NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(&name)
    .bind::<diesel::sql_types::Text, _>(&hash)
    .bind::<diesel::sql_types::Text, _>(&display_prefix(&secret))
    .bind::<diesel::sql_types::Text, _>(&scopes.to_string())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(parse_expiry(form.expires_in_days))
    .execute(&mut conn);

    if let Err(e) = inserted {
        log::error!("api_keys_create insert failed: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to create API key".to_string()));
    }

    crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
        state: &state,
        event_type: "api_key",
        actor_id: user_id,
        action: "api_key.create",
        target_type: Some("api_key"),
        target_id: None,
        success: true,
        details: Some(&format!("created key '{name}'")),
    });

    (
        StatusCode::OK,
        Html(format!(
            r#"<div class="api-key-created">
                <p>API key created — copy it now, it will not be shown again:</p>
                <code class="api-key-secret">{secret}</code>
                <p class="hint">Scopes: {scopes} · Expires: {expiry}</p>
                <button class="btn-secondary btn-sm" onclick="navigator.clipboard.writeText('{secret}'); this.textContent='Copied';">Copy</button>
            </div>"#,
            expiry = parse_expiry(form.expires_in_days)
                .map(|t| t.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "never".to_string()),
        )),
    )
}

/// PATCH-like rotate: revokes the current secret and issues a new one.
/// Implemented as POST /api/user/api-keys/{key_id}/rotate.
pub async fn api_keys_rotate(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(key_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let (secret, hash) = generate_api_key();
    let updated = diesel::sql_query(
        "UPDATE api_keys SET key_hash = $1, key_prefix = $2, last_used_at = last_used_at \
         WHERE id = $3 AND user_id = $4 AND is_active = true",
    )
    .bind::<diesel::sql_types::Text, _>(&hash)
    .bind::<diesel::sql_types::Text, _>(&display_prefix(&secret))
    .bind::<diesel::sql_types::Uuid, _>(key_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .execute(&mut conn);

    match updated {
        Ok(1) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "api_key",
                actor_id: user_id,
                action: "api_key.rotate",
                target_type: Some("api_key"),
                target_id: Some(key_id),
                success: true,
                details: None,
            });
            (
                StatusCode::OK,
                Html(format!(
                    r#"<div class="api-key-created">
                        <p>API key rotated — new secret shown once:</p>
                        <code class="api-key-secret">{secret}</code>
                        <button class="btn-secondary btn-sm" onclick="navigator.clipboard.writeText('{secret}'); this.textContent='Copied';">Copy</button>
                    </div>"#
                )),
            )
        }
        Ok(_) => (StatusCode::NOT_FOUND, Html("API key not found or already revoked".to_string())),
        Err(e) => {
            log::error!("api_keys_rotate failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to rotate API key".to_string()))
        }
    }
}

/// DELETE /api/user/api-keys/{key_id} — soft revoke.
pub async fn api_keys_delete(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(key_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let updated = diesel::sql_query(
        "UPDATE api_keys SET is_active = false, revoked_at = NOW() \
         WHERE id = $1 AND user_id = $2 AND is_active = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(key_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .execute(&mut conn);

    match updated {
        Ok(1) => {
            crate::audit_log::record_audit_event(crate::audit_log::AuditEvent {
                state: &state,
                event_type: "api_key",
                actor_id: user_id,
                action: "api_key.revoke",
                target_type: Some("api_key"),
                target_id: Some(key_id),
                success: true,
                details: None,
            });
            (StatusCode::OK, Html(String::new()))
        }
        Ok(_) => (StatusCode::NOT_FOUND, Html("API key not found".to_string())),
        Err(e) => {
            log::error!("api_keys_delete failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to revoke API key".to_string()))
        }
    }
}

/// Resolves a `gb_` secret to the owning user + scopes, for use by the auth
/// middleware. Returns None when the key is unknown, inactive or expired.
pub fn resolve_api_key(
    pool: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    secret: &str,
) -> Option<(Uuid, serde_json::Value)> {
    if !secret.starts_with(KEY_PREFIX) {
        return None;
    }
    let mut conn = pool.get().ok()?;
    let hash = hash_secret(secret);

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct KeyRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        user_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Jsonb)]
        scopes: serde_json::Value,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
        expires_at: Option<chrono::DateTime<Utc>>,
    }

    let row: KeyRow = diesel::sql_query(
        "SELECT user_id, scopes, expires_at FROM api_keys WHERE key_hash = $1 AND is_active = true",
    )
    .bind::<diesel::sql_types::Text, _>(&hash)
    .get_result(&mut conn)
    .ok()?;

    if is_expired(row.expires_at) {
        return None;
    }

    // Bump last_used_at (best-effort, non-fatal).
    let _ = diesel::sql_query("UPDATE api_keys SET last_used_at = NOW() WHERE key_hash = $1")
        .bind::<diesel::sql_types::Text, _>(&hash)
        .execute(&mut conn);

    Some((row.user_id, row.scopes))
}

/// Parses a comma-separated scope string into a JSON array.
/// Falls back to the default `["user"]` scope.
fn parse_scopes(raw: Option<&str>) -> serde_json::Value {
    let scopes: Vec<String> = raw
        .map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if scopes.is_empty() {
        serde_json::json!(["user"])
    } else {
        serde_json::Value::Array(scopes.into_iter().map(serde_json::Value::String).collect())
    }
}

