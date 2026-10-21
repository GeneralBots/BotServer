use crate::schema::attendance_webhooks;
use crate::AttendanceConfig;
use axum::{extract::{Path, State}, http::StatusCode, Json};
use chrono::Utc;
use diesel::prelude::*;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = attendance_webhooks)]
pub struct AttendanceWebhook {
    pub id: Uuid,
    pub org_id: Uuid,
    pub branch_id: Uuid,
    pub webhook_url: String,
    pub events: Option<Vec<Option<String>>>,
    pub is_active: Option<bool>,
    pub secret_key: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookRequest {
    pub webhook_url: String,
    pub events: Vec<String>,
    pub secret_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWebhookRequest {
    pub webhook_url: Option<String>,
    pub events: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub secret_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: String,
    pub bot_id: Uuid,
    pub data: serde_json::Value,
}

fn calculate_hmac_signature(secret: &str, payload: &str) -> String {
    // #1368 — HMAC accepts a key of any length; a construction failure means
    // the crypto backend is unusable. Fail closed with an empty signature
    // instead of aborting the process.
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(e) => {
            log::error!("HMAC signature unavailable: {e}");
            return String::new();
        }
    };
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

pub fn emit_webhook_event(
    conn: &mut diesel::PgConnection,
    bot_id: Uuid,
    event: &str,
    data: serde_json::Value,
) {
    let branch_id = {
        use crate::schema::bots;
        bots::table
            .filter(bots::id.eq(bot_id))
            .select(bots::branch_id)
            .first::<Uuid>(conn)
            .unwrap_or(Uuid::nil())
    };
    let webhooks: Vec<AttendanceWebhook> = attendance_webhooks::table
        .filter(attendance_webhooks::branch_id.eq(branch_id))
        .filter(attendance_webhooks::is_active.eq(true))
        .load(conn)
        .unwrap_or_default();

    for webhook in webhooks {
        let event_str = event.to_string();
        if let Some(ref events) = webhook.events {
            if !events.iter().flatten().any(|e| e == &event_str) {
                continue;
            }
        }
        let payload = WebhookPayload {
            event: event_str,
            timestamp: Utc::now().to_rfc3339(),
            bot_id,
            data: data.clone(),
        };
        let payload_json = serde_json::to_string(&payload).unwrap_or_default();
        let mut request = reqwest::Client::new()
            .post(&webhook.webhook_url)
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(5))
            .body(payload_json.clone());
        if let Some(ref secret) = webhook.secret_key {
            let signature = calculate_hmac_signature(secret, &payload_json);
            request = request.header("X-Webhook-Signature", signature);
        }
        let url_for_log = webhook.webhook_url.clone();
        let event_for_log = event.to_string();
        tokio::spawn(async move {
            if let Err(e) = request.send().await {
                log::error!("Failed to emit webhook {}: {}", url_for_log, e);
            } else {
                log::info!("Webhook emitted successfully: {} event={}", url_for_log, event_for_log);
            }
        });
    }
}

fn get_bot_context(config: &AttendanceConfig) -> Uuid {
    let Ok(mut conn) = config.pool.get() else {
        return Uuid::nil();
    };
    
    (config.get_default_bot)(&mut conn)
}

fn org_for_branch(config: &AttendanceConfig, branch_id: Uuid) -> Uuid {
    let Ok(mut conn) = config.pool.get() else {
        return Uuid::nil();
    };
    crate::schema::bots::table
        .filter(crate::schema::bots::branch_id.eq(branch_id))
        .select(crate::schema::bots::org_id)
        .first(&mut conn)
        .unwrap_or(Uuid::nil())
}

pub async fn list_webhooks(
    State(config): State<Arc<AttendanceConfig>>,
) -> Result<Json<Vec<AttendanceWebhook>>, (StatusCode, String)> {
    let mut conn = config.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
    let branch_id = get_bot_context(&config);
    let webhooks: Vec<AttendanceWebhook> = attendance_webhooks::table
        .filter(attendance_webhooks::branch_id.eq(branch_id))
        .order(attendance_webhooks::created_at.desc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;
    Ok(Json(webhooks))
}

pub async fn create_webhook(
    State(config): State<Arc<AttendanceConfig>>,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<Json<AttendanceWebhook>, (StatusCode, String)> {
    let mut conn = config.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
    let branch_id = get_bot_context(&config);
    let id = Uuid::new_v4();
    let now = Utc::now();
    let webhook = AttendanceWebhook {
        id,
        branch_id,
        is_active: Some(true),
        created_at: now,
        updated_at: Some(now),
        webhook_url: req.webhook_url,
        events: Some(req.events.into_iter().map(Some).collect()),
        secret_key: req.secret_key,
        org_id: org_for_branch(&config, branch_id),
    };
    diesel::insert_into(attendance_webhooks::table)
        .values(&webhook)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;
    Ok(Json(webhook))
}

pub async fn get_webhook(
    State(config): State<Arc<AttendanceConfig>>,
    Path(id): Path<Uuid>,
) -> Result<Json<AttendanceWebhook>, (StatusCode, String)> {
    let mut conn = config.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
    let webhook: AttendanceWebhook = attendance_webhooks::table
        .filter(attendance_webhooks::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Webhook not found".to_string()))?;
    Ok(Json(webhook))
}

pub async fn update_webhook(
    State(config): State<Arc<AttendanceConfig>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWebhookRequest>,
) -> Result<Json<AttendanceWebhook>, (StatusCode, String)> {
    let mut conn = config.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
    let now = Utc::now();
    if let Some(new_url) = req.webhook_url {
        diesel::update(attendance_webhooks::table.filter(attendance_webhooks::id.eq(id)))
            .set(attendance_webhooks::webhook_url.eq(new_url))
            .execute(&mut conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(new_events) = req.events {
        let events_value: Vec<Option<String>> = new_events.into_iter().map(Some).collect();
        diesel::update(attendance_webhooks::table.filter(attendance_webhooks::id.eq(id)))
            .set(attendance_webhooks::events.eq(Some(events_value)))
            .execute(&mut conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(active) = req.is_active {
        diesel::update(attendance_webhooks::table.filter(attendance_webhooks::id.eq(id)))
            .set(attendance_webhooks::is_active.eq(Some(active)))
            .execute(&mut conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(new_secret) = req.secret_key {
        diesel::update(attendance_webhooks::table.filter(attendance_webhooks::id.eq(id)))
            .set(attendance_webhooks::secret_key.eq(Some(new_secret)))
            .execute(&mut conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    diesel::update(attendance_webhooks::table.filter(attendance_webhooks::id.eq(id)))
        .set(attendance_webhooks::updated_at.eq(now))
        .execute(&mut conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    let webhook: AttendanceWebhook = attendance_webhooks::table
        .filter(attendance_webhooks::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Webhook not found".to_string()))?;
    Ok(Json(webhook))
}

pub async fn delete_webhook(
    State(config): State<Arc<AttendanceConfig>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = config.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
    diesel::delete(attendance_webhooks::table.filter(attendance_webhooks::id.eq(id)))
        .execute(&mut conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn test_webhook(
    State(config): State<Arc<AttendanceConfig>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = config.pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
    let webhook: AttendanceWebhook = attendance_webhooks::table
        .filter(attendance_webhooks::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Webhook not found".to_string()))?;
    let payload = WebhookPayload {
        event: "test".to_string(), timestamp: Utc::now().to_rfc3339(), bot_id: webhook.branch_id,
        data: serde_json::json!({ "message": "This is a test webhook" }),
    };
    let payload_json = serde_json::to_string(&payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization error: {e}")))?;
    let client = reqwest::Client::new();
    let mut request = client.post(&webhook.webhook_url);
    if let Some(ref secret) = webhook.secret_key {
        let signature = calculate_hmac_signature(secret, &payload_json);
        request = request.header("X-Webhook-Signature", signature);
    }
    request = request.header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .body(payload_json);
    match request.send().await {
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Ok(Json(serde_json::json!({ "success": status.is_success(), "status_code": status.as_u16(), "response": body })))
        }
        Err(e) => {
            log::error!("Webhook test failed: {}", e);
            Ok(Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botattendance::webhooks", None) })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_signature_generation() {
        let secret = "test-secret";
        let payload = r#"{"event":"test","data":{}}"#;
        let signature = calculate_hmac_signature(secret, payload);
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64);
    }
}
