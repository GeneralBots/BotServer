use chrono::Utc;
use diesel::RunQueryDsl;
use hmac::{Hmac, Mac};
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;

pub const WEBHOOK_VAULT_ROOT: &str = "secret/gbo/settings/webhooks";

pub fn webhook_vault_path(webhook_id: Uuid) -> String {
    format!("{WEBHOOK_VAULT_ROOT}/{webhook_id}")
}

pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Validates a webhook URL. HTTPS is required in production; private/local
/// destinations are always rejected to prevent SSRF.
pub fn validate_webhook_url(url: &str) -> Result<(), &'static str> {
    if url.trim().is_empty() {
        return Err("Webhook URL is required");
    }
    let is_prod = std::env::var("GB_ENV").map(|v| v == "production").unwrap_or(false);
    if is_prod && !url.starts_with("https://") {
        return Err("Webhook URL must use HTTPS in production");
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Webhook URL must start with http:// or https://");
    }
    let forbidden = ["localhost", "127.0.0.1", "0.0.0.0", "::1", "169.254.", "10.", "192.168."];
    if forbidden.iter().any(|f| url.contains(f)) {
        return Err("Webhook URL cannot point to localhost or private networks");
    }
    Ok(())
}

pub fn parse_events(raw: Option<&str>) -> serde_json::Value {
    let events: Vec<String> = raw
        .map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if events.is_empty() {
        serde_json::json!(["*"])
    } else {
        serde_json::Value::Array(events.into_iter().map(serde_json::Value::String).collect())
    }
}

/// HMAC-SHA256 v1 signature over the payload.
pub fn sign_payload(payload: &str, secret: &str) -> String {
    type HmacSha256 = Hmac<sha2::Sha256>;
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        log::error!("webhook sign: HMAC key rejected");
        return String::new();
    };
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    format!("v1={}", hex::encode(result.into_bytes()))
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

/// Delivers a payload with exponential backoff retries, recording every
/// attempt in `webhook_deliveries`. Runs in a background task.
pub async fn deliver_with_retries(
    state: Arc<AppState>,
    delivery_id: Uuid,
    url: String,
    payload: &str,
    secret: &str,
) {
    let max_attempts = 5;
    let mut attempt = 0;
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::error!("webhook deliver: client build failed: {e}");
            mark_delivery(&state, delivery_id, "failed", None, None, attempt, Some(&e.to_string()));
            return;
        }
    };

    loop {
        attempt += 1;
        let signature = sign_payload(payload, secret);
        let result = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &signature)
            .header("X-Webhook-Event", "test")
            .body(payload.to_string())
            .send()
            .await;

        match result {
            Ok(resp) => {
                let code = resp.status().as_u16();
                let ok = (200..300).contains(&code);
                let body = resp.text().await.unwrap_or_default();
                let status = if ok { "success" } else { "failed" };
                let err = if ok { None } else { Some(format!("HTTP {code}: {}", truncate(&body, 200))) };
                let should_retry = !ok && attempt < max_attempts;
                if should_retry {
                    mark_delivery(&state, delivery_id, "retrying", Some(code), Some(&body), attempt, err.as_deref());
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt as u32) * 5)).await;
                    continue;
                }
                mark_delivery(&state, delivery_id, status, Some(code), Some(&body), attempt, err.as_deref());
                return;
            }
            Err(e) => {
                let msg = format!("request error: {e}");
                if attempt < max_attempts {
                    mark_delivery(&state, delivery_id, "retrying", None, None, attempt, Some(&msg));
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt as u32) * 5)).await;
                    continue;
                }
                mark_delivery(&state, delivery_id, "failed", None, None, attempt, Some(&msg));
                return;
            }
        }
    }
}

fn mark_delivery(
    state: &Arc<AppState>,
    delivery_id: Uuid,
    status: &str,
    response_code: Option<u16>,
    response_body: Option<&str>,
    attempt: i32,
    error: Option<&str>,
) {
    let Some(mut conn) = state.conn.get().ok() else {
        log::warn!("webhook deliver: DB unavailable, delivery {delivery_id} not recorded");
        return;
    };
    let completed_at: Option<chrono::DateTime<Utc>> = match status {
        "success" | "failed" => Some(Utc::now()),
        _ => None,
    };
    let _ = diesel::sql_query(
        "UPDATE webhook_deliveries SET status = $1, response_code = $2, response_body = $3, \
         attempt = $4, error = $5, next_retry_at = $6, completed_at = $7 WHERE id = $8",
    )
    .bind::<diesel::sql_types::Text, _>(status)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Int4>, _>(response_code.map(i32::from))
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(response_body)
    .bind::<diesel::sql_types::Int4, _>(attempt)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(error)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(
        if status == "retrying" {
            Some(Utc::now() + chrono::Duration::seconds(60))
        } else {
            None
        },
    )
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(completed_at)
    .bind::<diesel::sql_types::Uuid, _>(delivery_id)
    .execute(&mut conn);
}
