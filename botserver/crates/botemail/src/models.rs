use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_types::{
    Bool, Integer, Nullable, Text, Timestamptz, Uuid as DieselUuid, Varchar,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;


pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub type GetDefaultBotFn = Arc<dyn Fn(&mut PgConnection) -> (Uuid, String) + Send + Sync>;

pub type SecretsProvider = Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync>;

#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
    pub secrets_provider: SecretsProvider,
}

#[derive(Debug, QueryableByName)]
pub struct EmailAccountBasicRow {
    #[diesel(sql_type = DieselUuid)]
    pub id: Uuid,
    #[diesel(sql_type = Text)]
    pub email: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub display_name: Option<String>,
    #[diesel(sql_type = Bool)]
    pub is_primary: bool,
    /// Failure reported by the last sync pass, surfaced next to the account so
    /// an unreachable mailbox is distinguishable from an empty one.
    #[diesel(sql_type = Nullable<Text>)]
    pub last_error: Option<String>,
}

#[derive(Debug, QueryableByName)]
pub struct ImapCredentialsRow {
    #[diesel(sql_type = Text)]
    pub imap_server: String,
    #[diesel(sql_type = Integer)]
    pub imap_port: i32,
    #[diesel(sql_type = Text)]
    pub username: String,
    #[diesel(sql_type = Text)]
    pub password_encrypted: String,
}

#[derive(Debug, QueryableByName)]
pub struct SmtpCredentialsRow {
    #[diesel(sql_type = Text)]
    pub email: String,
    #[diesel(sql_type = Text)]
    pub display_name: String,
    #[diesel(sql_type = Integer)]
    pub smtp_port: i32,
    #[diesel(sql_type = Text)]
    pub smtp_server: String,
    #[diesel(sql_type = Text)]
    pub username: String,
    #[diesel(sql_type = Text)]
    pub password_encrypted: String,
    #[diesel(sql_type = Text)]
    pub auth_mode: String,
}

#[derive(Debug, QueryableByName)]
pub struct EmailSearchRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub subject: String,
    #[diesel(sql_type = Text)]
    pub from_address: String,
    #[diesel(sql_type = Text)]
    pub to_addresses: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub body_text: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, QueryableByName, Serialize)]
pub struct EmailSignatureRow {
    #[diesel(sql_type = DieselUuid)]
    pub id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    pub user_id: Uuid,
    #[diesel(sql_type = Nullable<DieselUuid>)]
    pub bot_id: Option<Uuid>,
    #[diesel(sql_type = Varchar)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub content_html: String,
    #[diesel(sql_type = Text)]
    pub content_plain: String,
    #[diesel(sql_type = Bool)]
    pub is_default: bool,
    #[diesel(sql_type = Bool)]
    pub is_active: bool,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, QueryableByName)]
pub struct EmailAccountRow {
    #[diesel(sql_type = DieselUuid)]
    pub id: Uuid,
    #[diesel(sql_type = Text)]
    pub email: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub display_name: Option<String>,
    #[diesel(sql_type = Text)]
    pub imap_server: String,
    #[diesel(sql_type = Integer)]
    pub imap_port: i32,
    #[diesel(sql_type = Text)]
    pub smtp_server: String,
    #[diesel(sql_type = Integer)]
    pub smtp_port: i32,
    #[diesel(sql_type = Text)]
    pub username: String,
    #[diesel(sql_type = Text)]
    pub password_encrypted: String,
    #[diesel(sql_type = Bool)]
    pub is_primary: bool,
    #[diesel(sql_type = Bool)]
    pub is_active: bool,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    pub updated_at: DateTime<Utc>,
}

pub struct EmailError(pub String);

impl IntoResponse for EmailError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}

impl From<String> for EmailError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Debug, QueryableByName, Serialize)]
pub struct EmailMessageRow {
    #[diesel(sql_type = DieselUuid)]
    pub id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    pub account_id: Uuid,
    #[diesel(sql_type = Nullable<Varchar>)]
    pub message_id_header: Option<String>,
    #[diesel(sql_type = Nullable<Varchar>)]
    pub in_reply_to: Option<String>,
    #[diesel(sql_type = Text)]
    pub subject: String,
    #[diesel(sql_type = Text)]
    pub normalized_subject: String,
    #[diesel(sql_type = Varchar)]
    pub from_address: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub to_addresses: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub body_text: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub body_html: Option<String>,
    #[diesel(sql_type = Bool)]
    pub has_attachments: bool,
    #[diesel(sql_type = Varchar)]
    pub folder: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub uid: i64,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub flags: serde_json::Value,
    #[diesel(sql_type = Bool)]
    pub is_read: bool,
    #[diesel(sql_type = Bool)]
    pub is_flagged: bool,
    #[diesel(sql_type = Timestamptz)]
    pub received_at: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    pub synced_at: DateTime<Utc>,
}

#[derive(Debug, QueryableByName, Serialize)]
pub struct EmailAccountColorRow {
    #[diesel(sql_type = DieselUuid)]
    pub id: Uuid,
    #[diesel(sql_type = Text)]
    pub email: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub display_name: Option<String>,
    #[diesel(sql_type = Bool)]
    pub is_primary: bool,
}

pub struct EmailData {
    pub id: String,
    pub from_name: String,
    pub from_email: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub date: String,
    pub read: bool,
}

#[derive(Debug, diesel::QueryableByName)]
pub struct EmailSummary {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub from_name: String,
    #[diesel(sql_type = Text)]
    pub from_email: String,
    #[diesel(sql_type = Text)]
    pub subject: String,
    #[diesel(sql_type = Text)]
    pub preview: String,
    #[diesel(sql_type = Text)]
    pub date: String,
    #[diesel(sql_type = Bool)]
    pub read: bool,
}

pub struct EmailContent {
    pub id: String,
    pub from_name: String,
    pub from_email: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub date: String,
    pub read: bool,
}

/// Decodes a URL-safe base64 payload (JWT claims). Trailing padding is
/// stripped because JSON Web Tokens omit it and the decoder strictly walks
/// the base64url alphabet. `None` is returned for invalid input so that a
/// malformed token never aborts a request.
fn decode_jwt_payload(segment: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let input = segment.trim_end_matches('=');
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in input.as_bytes() {
        let v = {
            let i = TABLE.iter().position(|&x| x == b)?;
            i as u32
        };
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// Maps a raw identity subject to the stable database UUID used for mail
/// accounts. Zitadel/OIDC numeric user ids are not valid UUIDs; routing them
/// through `UUIDv5(zitadel:{id})` produces the same deterministic value that
/// the RBAC layer derives, keeping `user_email_accounts` in the caller scope.
pub fn stable_user_uuid(raw: &str) -> Uuid {
    match Uuid::parse_str(raw) {
        Ok(uuid) => uuid,
        Err(_) => Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("zitadel:{raw}").as_bytes()),
    }
}

/// Resolves the authenticated user id from the request headers.
///
/// Supported identities, in priority order:
/// 1. `X-User-ID` header, used by the chat/WhatsApp loopback executor that
///    cannot attach an `Authorization` header.
/// 2. Opaque suite session token (`gb_*`) resolved through the shared session
///    cache populated at login.
/// 3. Bearer JWT whose claims identify the user (`sub` / `user_id`).
///
/// Never falls back to `Uuid::nil()` for an authenticated request: if the
/// identity cannot be resolved this fails so callers can reject the request.
pub fn extract_user_from_session(headers: &HeaderMap) -> Result<Uuid, String> {
    if let Some(uid) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) {
        if let Ok(uuid) = Uuid::parse_str(uid) {
            return Ok(uuid);
        }
    }

    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Missing Authorization header".to_string())?;
    let token = auth
        .strip_prefix("Bearer ")
        .ok_or_else(|| "Authorization header is not a Bearer token".to_string())?;

    if !token.contains('.') {
        let entry = botsecurity_core::lookup_session_cache(token)
            .ok_or_else(|| "Unknown or expired session token".to_string())?;
        return Ok(stable_user_uuid(&entry.user_id));
    }

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Malformed JWT token".to_string());
    }
    let raw = decode_jwt_payload(parts[1])
        .ok_or_else(|| "Invalid JWT payload encoding".to_string())?;
    let claims: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|e| format!("Invalid JWT payload JSON: {e}"))?;
    let sub = claims
        .get("sub")
        .or_else(|| claims.get("user_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "JWT carries no identity claim".to_string())?;
    if sub.is_empty() {
        return Err("JWT identity claim is empty".to_string());
    }
    Ok(stable_user_uuid(sub))
}

#[derive(Debug, QueryableByName, serde::Serialize)]
pub struct EmailLabelRow {
    #[diesel(sql_type = DieselUuid)]
    pub id: Uuid,
    #[diesel(sql_type = Varchar)]
    pub name: String,
    #[diesel(sql_type = Varchar)]
    pub color: String,
}
