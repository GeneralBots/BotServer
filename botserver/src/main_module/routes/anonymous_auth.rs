use axum::{Json, extract::State, response::IntoResponse};
use std::sync::Arc;
use std::collections::HashMap;
use log::info;
use botcore::shared::state::AppState;

pub async fn anonymous_auth_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    let bot_name = params.get("bot_name").cloned().unwrap_or_default();
    let existing_session_id = params.get("session_id").cloned();
    let existing_user_id = params.get("user_id").cloned();

    // Resolve the real user from the suite session token (gb-access-token)
    // when present as `Authorization: Bearer <token>`.
    let (auth_user_id, auth_roles, is_authenticated) = resolve_session_user(&request);

    let user_id = if is_authenticated {
        auth_user_id.clone()
    } else {
        existing_user_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    };
    let session_id = existing_session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let session_uuid = match uuid::Uuid::parse_str(&session_id) {
        Ok(uuid) => uuid,
        Err(_) => uuid::Uuid::new_v4(),
    };
    let user_uuid = resolve_chat_user_uuid(&user_id);

    let found_bot_id = {
        let conn = state.conn.get().ok();
        if let Some(mut db_conn) = conn {
            use botcore::shared::models::schema::bots::dsl::*;
            use diesel::prelude::*;
            bots.filter(name.eq(&bot_name))
                .select(id)
                .first::<uuid::Uuid>(&mut db_conn)
                .ok()
                .unwrap_or_else(uuid::Uuid::nil)
        } else {
            uuid::Uuid::nil()
        }
    };

    let role = if is_authenticated && auth_roles.iter().any(|r| r.to_lowercase().contains("admin")) {
        crate::security::user_role::ROLE_ADMIN.to_string()
    } else {
        crate::security::user_role::resolve_user_role(&state.conn, user_uuid)
    };

    let mut final_session_id = session_id.clone();
    {
        let mut sm = state.session_manager.lock().await;
        sm.get_or_create_anonymous_user(Some(user_uuid)).ok();
        let session = sm.get_or_create_session_by_id(
            session_uuid, user_uuid, found_bot_id, "Anonymous Chat"
        );
        if let Ok(sess) = session {
            final_session_id = sess.id.to_string();
        }
    }

    info!("Anonymous auth for bot: {}, session: {}, authenticated: {}", bot_name, final_session_id, is_authenticated);

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "user_id": user_id,
            "session_id": final_session_id,
            "bot_id": found_bot_id,
            "bot_name": bot_name,
            "status": if is_authenticated { "authenticated" } else { "anonymous" },
            "role": role,
            "is_authenticated": is_authenticated
        })),
    )
}

/// Resolves the chat user UUID from a raw user identifier: parses it as a
/// UUID when possible, otherwise derives a deterministic UUID from non-UUID
/// identifiers (e.g. Zitadel numeric user_id / sub claim).
pub(crate) fn resolve_chat_user_uuid(user_id: &str) -> uuid::Uuid {
    if let Ok(uuid) = uuid::Uuid::parse_str(user_id) {
        return uuid;
    }
    uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_DNS,
        format!("zitadel:{user_id}").as_bytes(),
    )
}

pub(crate) fn resolve_session_user(
    request: &axum::extract::Request,
) -> (String, Vec<String>, bool) {    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let token = match token {
        Some(t) => t,
        None => return (String::new(), Vec::new(), false),
    };

    let cache = match botcoredirectory::auth_routes::SESSION_CACHE.try_read() {
        Ok(c) => c,
        Err(_) => return (String::new(), Vec::new(), false),
    };
    if let Some(user_data) = cache.get(&token) {
        return (
            user_data.user_id.clone(),
            user_data.roles.clone(),
            true,
        );
    }
    drop(cache);

    // In-memory cache miss — rehydrate from the persisted login_sessions
    // table (parity with the auth middleware lookup in main.rs) so tokens
    // minted before the last restart keep their identity.
    if let Some(user_data) = botcoredirectory::auth_routes::session_from_persisted(&token) {
        return (
            user_data.user_id.clone(),
            user_data.roles.clone(),
            true,
        );
    }

    // Not an SSO-session token — try the cloud management JWT (signed with the
    // SaaS secret, subject = Zitadel user id). Verified signature required;
    // roles are resolved later through RBAC group membership.
    resolve_cloud_jwt_user(&token)
}

/// #1364 — verifies a bearer token as a cloud management JWT and maps it to
/// a session user for `/api/auth/me`. Same HMAC check as
/// `resolve_cloud_jwt_user`, but returns the full identity the sidebar
/// needs (name/email/org from the JWT claims).
pub(crate) fn cloud_jwt_session_user(
    token: &str,
) -> Option<botcoredirectory::auth_routes::SessionUserData> {
    let secret = crate::main_module::directory_setup::resolve_saas_jwt_secret();

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected = base64_url_decode_impl(parts[2])?;

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(signing_input.as_bytes());
    if mac.finalize().into_bytes().as_slice() != expected.as_slice() {
        return None;
    }
    let payload_json = base64_url_decode_impl(parts[1])?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_json).ok()?;
    let sub = payload.get("sub").and_then(|v| v.as_str()).unwrap_or("");
    if sub.is_empty() {
        return None;
    }
    // Honor the JWT `exp` claim when present (epoch seconds).
    if let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) {
        if chrono::Utc::now().timestamp() >= exp {
            return None;
        }
    }
    let email = payload
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = email.split('@').next().unwrap_or("User").to_string();
    Some(botcoredirectory::auth_routes::SessionUserData {
        user_id: sub.to_string(),
        username: name,
        email,
        first_name: None,
        last_name: None,
        display_name: payload
            .get("name")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        organization_id: payload
            .get("org_id")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        roles: Vec::new(),
        bucket: None,
        created_at: chrono::Utc::now().timestamp(),
    })
}

/// Validates a cloud management JWT (HMAC-SHA256, SaaS secret) and returns
/// the verified subject (Zitadel user id) as the chat user identity.
fn resolve_cloud_jwt_user(token: &str) -> (String, Vec<String>, bool) {
    let secret = crate::main_module::directory_setup::resolve_saas_jwt_secret();

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return (String::new(), Vec::new(), false);
    }
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected = match base64_url_decode_impl(parts[2]) {
        Some(sig) => sig,
        None => return (String::new(), Vec::new(), false),
    };

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return (String::new(), Vec::new(), false),
    };
    mac.update(signing_input.as_bytes());
    let sig = mac.finalize().into_bytes();
    if sig.as_slice() != expected.as_slice() {
        return (String::new(), Vec::new(), false);
    }

    let payload_json = match base64_url_decode_impl(parts[1]) {
        Some(bytes) => bytes,
        None => return (String::new(), Vec::new(), false),
    };
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&payload_json) else {
        return (String::new(), Vec::new(), false);
    };
    let sub = payload.get("sub").and_then(|v| v.as_str()).unwrap_or("");
    if sub.is_empty() {
        return (String::new(), Vec::new(), false);
    }
    info!("Cloud JWT verified for chat session: sub={}...", botlib::text::truncate_at_char_boundary(sub, 24));
    (sub.to_string(), Vec::new(), true)
}

fn base64_url_decode_impl(input: &str) -> Option<Vec<u8>> {
    use base64::{Engine as _, engine::general_purpose};
    let raw = input.replace('-', "+").replace('_', "/");
    let raw = match raw.len() % 4 {
        2 => format!("{raw}=="),
        3 => format!("{raw}="),
        0 => raw,
        _ => return None,
    };
    general_purpose::STANDARD.decode(raw).ok()
}
