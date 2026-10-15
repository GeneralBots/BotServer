//! Mailbox OAuth2 consent flow.
//!
//! Microsoft 365, Outlook.com and Gmail no longer accept a static password over
//! IMAP/SMTP, so those mailboxes are connected by sending the user to the
//! provider's consent screen. The provider then redirects back to the callback
//! below.
//!
//! The pending flow travels in a signed `state` parameter, so the callback can
//! trust the user and mailbox it carries without keeping a server-side session.
//! The signature uses `MAIL_OAUTH_STATE_SECRET`; when that is not configured the
//! flow refuses to start rather than trusting an unsigned state.

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use diesel::prelude::*;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{extract_user_from_session, AppState, EmailError};
use crate::oauth::{consent_url, encode_secret, exchange_code, MailOAuthProvider, TokenSet};
use crate::types::ApiResponse;

/// How long a consent flow may stay pending before its state is refused.
const STATE_TTL_SECS: i64 = 600;

/// Field separator for the signed state payload. Neither an address nor a URL
/// may contain it, which keeps positional parsing unambiguous.
const STATE_SEPARATOR: char = '|';

#[derive(Debug, Deserialize)]
pub struct OAuthStartRequest {
    pub provider: String,
    /// Mailbox address being connected. Both providers use it as the username.
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OAuthStartResponse {
    pub url: String,
}

fn state_secret() -> Option<String> {
    std::env::var("MAIL_OAUTH_STATE_SECRET")
        .ok()
        .filter(|value| !value.is_empty())
}

/// Builds the absolute callback URI the provider must redirect to. It has to
/// match the URI registered with the provider byte for byte, so the configured
/// base takes precedence over the request headers.
fn callback_url(headers: &axum::http::HeaderMap) -> String {
    if let Ok(base) = std::env::var("MAIL_OAUTH_REDIRECT_BASE") {
        let trimmed = base.trim_end_matches('/');
        if !trimmed.is_empty() {
            return format!("{trimmed}/api/email/oauth/callback");
        }
    }
    let host = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("localhost:8080");
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("https");
    format!("{scheme}://{host}/api/email/oauth/callback")
}

fn sign_state(payload: &str, secret: &str) -> String {
    let material = format!("{payload}{STATE_SEPARATOR}{}", Utc::now().timestamp());
    let signature = hmac_sha256_hex(&material, secret);
    format!(
        "{}.{}",
        general_purpose::URL_SAFE_NO_PAD.encode(material.as_bytes()),
        signature
    )
}

/// Verifies the signature and lifetime of a state parameter, returning the user
/// id, provider and mailbox address it carries.
fn verify_state(state: &str, secret: &str) -> Option<(Uuid, String, String, String)> {
    let (encoded, signature) = state.split_once('.')?;
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(encoded).ok()?;
    let material = String::from_utf8(decoded).ok()?;

    let expected = hmac_sha256_hex(&material, secret);
    if !constant_time_eq(signature, &expected) {
        return None;
    }

    let (payload, issued_at) = material.rsplit_once(STATE_SEPARATOR)?;
    let issued_at: i64 = issued_at.parse().ok()?;
    if Utc::now().timestamp() - issued_at > STATE_TTL_SECS {
        return None;
    }

    let fields: Vec<&str> = payload.splitn(4, STATE_SEPARATOR).collect();
    let (user_id, provider, email, redirect_uri) = match fields.as_slice() {
        [user_id, provider, email, redirect_uri] => (*user_id, *provider, *email, *redirect_uri),
        _ => return None,
    };
    let user_id = Uuid::parse_str(user_id).ok()?;
    Some((
        user_id,
        provider.to_string(),
        email.to_string(),
        redirect_uri.to_string(),
    ))
}

fn hmac_sha256_hex(value: &str, secret: &str) -> String {
    match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(mut mac) => {
            mac.update(value.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        }
        Err(_) => String::new(),
    }
}

/// Compares two hex signatures without leaking their contents through timing.
fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

/// Returns the provider consent URL the browser should be sent to.
pub async fn start_mail_oauth(
    headers: axum::http::HeaderMap,
    Json(request): Json<OAuthStartRequest>,
) -> Result<Json<ApiResponse<OAuthStartResponse>>, EmailError> {
    let Ok(user_id) = extract_user_from_session(&headers) else {
        return Err(EmailError("Authentication required".to_string()));
    };
    let Some(provider) = MailOAuthProvider::parse(&request.provider) else {
        return Err(EmailError(format!(
            "Unsupported OAuth2 provider '{}'",
            request.provider
        )));
    };
    let email = request.email.trim().to_string();
    if email.is_empty() {
        return Err(EmailError("The mailbox address is required".to_string()));
    }
    let Some(secret) = state_secret() else {
        return Err(EmailError(
            "MAIL_OAUTH_STATE_SECRET is not configured, so the consent flow cannot be signed"
                .to_string(),
        ));
    };
    // Only the client id is needed here; the secret is used by the callback
    // when the authorization code is exchanged.
    let Some(client_id) = provider.credentials().map(|credentials| credentials.0) else {
        return Err(EmailError(format!(
            "No OAuth2 client credentials configured for {}",
            provider.as_str()
        )));
    };

    let redirect_uri = callback_url(&headers);
    let provider_name = provider.as_str();
    let payload = format!(
        "{user_id}{STATE_SEPARATOR}{provider_name}{STATE_SEPARATOR}{email}{STATE_SEPARATOR}{redirect_uri}"
    );
    let signed_state = sign_state(&payload, &secret);
    let url = consent_url(provider, &client_id, &redirect_uri, &signed_state);

    // The client id is public; the secret stays on the server and is never part
    // of the response.
    Ok(Json(ApiResponse {
        success: true,
        data: Some(OAuthStartResponse { url }),
        message: None,
    }))
}

/// Receives the provider redirect, exchanges the code and stores the account.
pub async fn mail_oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<OAuthCallbackParams>,
) -> Response {
    if let Some(error) = params.error {
        let detail = params.error_description.unwrap_or_default();
        return EmailError(format!("The provider refused the consent request: {error} {detail}"))
            .into_response();
    }
    let (Some(code), Some(signed_state)) = (params.code, params.state) else {
        return EmailError(
            "The consent response is missing the code or state parameter".to_string(),
        )
        .into_response();
    };
    let Some(secret) = state_secret() else {
        return EmailError("MAIL_OAUTH_STATE_SECRET is not configured".to_string())
            .into_response();
    };
    let Some((user_id, provider_name, email, redirect_uri)) = verify_state(&signed_state, &secret)
    else {
        return EmailError(
            "The consent response is expired or its signature is invalid".to_string(),
        )
        .into_response();
    };
    let Some(provider) = MailOAuthProvider::parse(&provider_name) else {
        return EmailError(format!("Unsupported OAuth2 provider '{provider_name}'")).into_response();
    };
    let Some((client_id, client_secret)) = provider.credentials() else {
        return EmailError(format!(
            "No OAuth2 client credentials configured for {}",
            provider.as_str()
        ))
        .into_response();
    };

    let tokens = match exchange_code(
        provider,
        &client_id,
        &client_secret,
        &code,
        &redirect_uri,
    )
    .await
    {
        Ok(tokens) => tokens,
        Err(e) => return EmailError(e).into_response(),
    };

    let pool = state.pool.clone();
    let stored = tokio::task::spawn_blocking(move || -> Result<Uuid, String> {
        let mut conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;
        upsert_oauth_account(&mut conn, user_id, provider, &email, &tokens)
    })
    .await;

    match stored {
        Ok(Ok(_account_id)) => Redirect::to("/mail").into_response(),
        Ok(Err(e)) => EmailError(e).into_response(),
        Err(e) => EmailError(format!("Task join error: {e}")).into_response(),
    }
}

#[derive(Debug, diesel::QueryableByName)]
struct AccountIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
}

/// Creates the account for a freshly consented mailbox, or refreshes the tokens
/// of the account already connected for that address so the same mailbox is
/// never duplicated.
fn upsert_oauth_account(
    conn: &mut diesel::PgConnection,
    user_id: Uuid,
    provider: MailOAuthProvider,
    email: &str,
    tokens: &TokenSet,
) -> Result<Uuid, String> {
    let access_token = encode_secret(&tokens.access_token);
    let refresh_token = tokens.refresh_token.as_deref().map(encode_secret);

    let existing = diesel::sql_query(
        "SELECT id FROM user_email_accounts \
         WHERE user_id = $1 AND email = $2 AND is_active = true LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(email)
    .get_result::<AccountIdRow>(conn)
    .optional()
    .map_err(|e| format!("Failed to look up the mailbox account: {e}"))?
    .map(|row| row.id);

    if let Some(account_id) = existing {
        diesel::sql_query(
            "UPDATE user_email_accounts SET auth_mode = 'oauth2', oauth_provider = $1, \
             access_token_encrypted = $2, token_expires_at = $3, \
             refresh_token_encrypted = COALESCE($4, refresh_token_encrypted), \
             last_error = NULL, last_error_at = NULL, updated_at = now() WHERE id = $5",
        )
        .bind::<diesel::sql_types::Text, _>(provider.as_str())
        .bind::<diesel::sql_types::Text, _>(access_token)
        .bind::<diesel::sql_types::Timestamptz, _>(tokens.expires_at)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(refresh_token.as_deref())
        .bind::<diesel::sql_types::Uuid, _>(account_id)
        .execute(conn)
        .map_err(|e| format!("Failed to update the mailbox account: {e}"))?;
        return Ok(account_id);
    }

    let account_id = Uuid::new_v4();
    diesel::sql_query(
        "INSERT INTO user_email_accounts \
         (id, user_id, email, display_name, imap_server, imap_port, smtp_server, smtp_port, \
          username, password_encrypted, is_primary, is_active, auth_mode, oauth_provider, \
          access_token_encrypted, token_expires_at, refresh_token_encrypted) \
         VALUES ($1, $2, $3, $4, $5, 993, $6, 587, $3, '', false, true, 'oauth2', $7, $8, $9, $10)",
    )
    .bind::<diesel::sql_types::Uuid, _>(account_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(email)
    .bind::<diesel::sql_types::Text, _>(email)
    .bind::<diesel::sql_types::Text, _>(provider.default_imap_server())
    .bind::<diesel::sql_types::Text, _>(provider.default_smtp_server())
    .bind::<diesel::sql_types::Text, _>(provider.as_str())
    .bind::<diesel::sql_types::Text, _>(access_token)
    .bind::<diesel::sql_types::Timestamptz, _>(tokens.expires_at)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(refresh_token.as_deref())
    .execute(conn)
    .map_err(|e| format!("Failed to store the mailbox account: {e}"))?;

    Ok(account_id)
}
