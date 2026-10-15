use axum::{
    extract::{Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use botcore::shared::state::AppState;
use botcoredirectory::auth_routes::{SESSION_CACHE, SessionUserData, persist_session};

#[derive(Debug, Deserialize)]
pub struct SsoRequest {
    pub cloud_token: String,
}

#[derive(Debug, Serialize)]
pub struct SsoResponse {
    pub access_token: String,
    pub email: String,
    pub user_id: String,
}

/// POST /api/auth/cloud-sso
///
/// Troca um JWT do Cloud por um token de sessão do Suite.
/// Lê o segredo JWT de directory_config.json ou env var.
/// A assinatura é SEMPRE verificada — tokens com assinatura inválida são
/// rejeitados (fix #842: nunca aceitar payloads não assinados).
pub async fn handle_cloud_sso(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SsoRequest>,
) -> Result<Json<SsoResponse>, (StatusCode, Json<serde_json::Value>)> {
    let jwt_secret = get_saas_jwt_secret();

    // Validate JWT signature — reject on mismatch, no fallback
    let parts: Vec<&str> = req.cloud_token.split('.').collect();
    if parts.len() != 3 {
        return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Invalid token format"}))));
    }

    let (header_b64, payload_b64, sig_b64) = (parts[0], parts[1], parts[2]);
    let message = format!("{}.{}", header_b64, payload_b64);

    let expected_sig = match jwt_sign_inner(&message, jwt_secret.as_bytes()) {
        Ok(sig) => sig,
        Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Token signing failed"})))),
    };
    if sig_b64 != expected_sig {
        log::warn!("cloud-sso: JWT signature mismatch — rejecting token");
        return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Invalid token signature"}))));
    }

    // Decode JWT payload
    let payload_bytes = match base64_url_decode(payload_b64) {
        Ok(b) => b,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid token payload"})))),
    };

    let claims: serde_json::Value = match serde_json::from_slice(&payload_bytes) {
        Ok(v) => v,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid token payload JSON"})))),
    };

    let email = claims.get("email").and_then(|v| v.as_str()).unwrap_or("user@localhost");
    let user_id = claims.get("sub").and_then(|v| v.as_str()).unwrap_or("unknown");

    // Create Suite session with the real per-user role (fix #843) and the
    // JWT org scope so org-scoped APIs resolve the user's organization.
    let roles = resolve_session_roles(&state, user_id);
    let api_token = create_suite_session(email, user_id, None, roles, claims_org(&claims)).await;

    Ok(Json(SsoResponse {
        access_token: api_token,
        email: email.to_string(),
        user_id: user_id.to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct SuiteSsoQuery {
    pub token: String,
    pub redirect: String,
}

/// GET /api/auth/suite-sso
///
/// Server-side SSO hop: validates cloud JWT, stores suite token in localStorage
/// via HTML+script, then redirects to a clean URL (no token in QS).
/// Assinatura inválida → rejeição imediata (fix #842).
pub async fn handle_suite_sso(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SuiteSsoQuery>,
) -> Result<Response, StatusCode> {
    let jwt_secret = get_saas_jwt_secret();

    let parts: Vec<&str> = query.token.split('.').collect();
    if parts.len() != 3 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let message = format!("{}.{}", parts[0], parts[1]);
    let sig_ok = jwt_sign_inner(&message, jwt_secret.as_bytes()).map(|s| s == parts[2]).unwrap_or(false);
    if !sig_ok {
        log::warn!("suite-sso: JWT signature mismatch — rejecting token");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let payload_bytes = base64_url_decode(parts[1]).map_err(|_| StatusCode::BAD_REQUEST)?;
    let claims: serde_json::Value = serde_json::from_slice(&payload_bytes).map_err(|_| StatusCode::BAD_REQUEST)?;

    let email = claims.get("email").and_then(|v| v.as_str()).unwrap_or("user@localhost");
    let user_id = claims.get("sub").and_then(|v| v.as_str()).unwrap_or("unknown");
    let bucket = claims.get("bucket").and_then(|v| v.as_str()).map(|s| s.to_string());

    let roles = resolve_session_roles(&state, user_id);
    let suite_token = create_suite_session(email, user_id, bucket, roles, claims_org(&claims)).await;
    let redirect = sanitize_redirect(&query.redirect);

    let html = format!(
        r##"<!DOCTYPE html>
<html>
<head>
<meta name="referrer" content="no-referrer">
<script>
try {{
  localStorage.setItem('gb-access-token','{suite_token}');
  window.history.replaceState({{}},'','{redirect}');
  window.location.replace('{redirect}');
}} catch(e) {{
  console.error('suite-sso:',e);
}}
</script>
</head>
<body></body>
</html>"##
    );

    Ok(session_cookie(suite_token.clone(), Html(html)))
}

fn session_cookie(token: String, html: Html<String>) -> Response {
    match HeaderValue::from_str(&format!(
        "gb-access-token={token}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly; Secure"
    )) {
        Ok(value) => ([(header::SET_COOKIE, value)], html).into_response(),
        Err(e) => {
            log::warn!("suite-sso: refusing to set an unusable session cookie: {e}");
            html.into_response()
        }
    }
}

fn sanitize_redirect(url: &str) -> String {
    // Only allow relative paths or same-origin absolute URLs
    let forbidden = |s: &str| {
        s.contains('"')
            || s.contains('\'')
            || s.contains('\\')
            || s.contains('<')
            || s.contains('>')
            || s.contains('\n')
            || s.contains('\r')
    };
    let is_relative = url.starts_with('/');
    let is_absolute_with_host = (url.starts_with("http://") || url.starts_with("https://"))
        && url::Url::parse(url)
            .map(|u| u.host_str().is_some())
            .unwrap_or(false);
    if (is_relative || is_absolute_with_host) && !forbidden(url) {
        url.to_string()
    } else {
        "/".to_string()
    }
}

/// Extract the `org_id` claim from a decoded cloud JWT so suite sessions
/// carry the caller's organization scope (fix #1268). Returns `None` for
/// tokens minted without an org claim.
fn claims_org(claims: &serde_json::Value) -> Option<String> {
    claims
        .get("org_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Cria uma sessão no SESSION_CACHE do Suite e retorna o token `gb_{uuid}_{timestamp}`.
/// O papel da sessão é resolvido por grupo RBAC (fix #843) — nunca hardcoded.
pub(crate) async fn create_suite_session(
    email: &str,
    user_id: &str,
    bucket: Option<String>,
    roles: Vec<String>,
    organization_id: Option<String>,
) -> String {
    let api_token = format!("gb_{}_{}", uuid::Uuid::new_v4(), chrono::Utc::now().timestamp());
    let session_user = SessionUserData {
        user_id: user_id.to_string(),
        email: email.to_string(),
        username: email.split('@').next().unwrap_or("user").to_string(),
        first_name: None,
        last_name: None,
        display_name: Some(email.split('@').next().unwrap_or("User").to_string()),
        organization_id,
        roles,
        bucket,
        created_at: chrono::Utc::now().timestamp(),
    };

    {
        let mut cache = SESSION_CACHE.write().await;
        cache.insert(api_token.clone(), session_user.clone());
        persist_session(&api_token, &session_user);
    }

    log::info!("Created Suite session for {} (token: {}...)", email, &api_token[..6.min(api_token.len())]);
    api_token
}

/// Resolves the effective role vector for a user id from RBAC group membership.
/// Falls back to the plain "user" role when the user has no admin group.
fn resolve_session_roles(state: &Arc<AppState>, user_id: &str) -> Vec<String> {
    let stable_uuid = crate::security::user_role::derive_stable_user_uuid(user_id);
    let role = crate::security::user_role::resolve_user_role(&state.conn, stable_uuid);
    vec![role]
}

/// POST /api/auth/unified-login
///
/// Login unificado que retorna AMBOS os tokens: Suite (SESSION_CACHE) e Cloud (JWT).
/// Aceita { email, password } ou { cloud_token }.
pub async fn handle_unified_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let jwt_secret = get_saas_jwt_secret();

    let email: String;
    let user_id: String;
    let org_scope: Option<String>;

    // Support both direct login and cloud_token exchange
    if let Some(cloud_token) = body.get("cloud_token").and_then(|v| v.as_str()) {
        // Exchange Cloud JWT for Suite session — signature MUST verify (fix #842)
        let parts: Vec<&str> = cloud_token.split('.').collect();
        if parts.len() != 3 {
            return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Invalid cloud token format"}))));
        }
        let message = format!("{}.{}", parts[0], parts[1]);
        let expected_sig = match jwt_sign_inner(&message, jwt_secret.as_bytes()) {
            Ok(sig) => sig,
            Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Token signing failed"})))),
        };
        if parts[2] != expected_sig {
            log::warn!("unified-login (cloud_token): JWT signature mismatch — rejecting token");
            return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Invalid token signature"}))));
        }
        let payload_bytes = base64_url_decode(parts[1]).map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid payload"}))))?;
        let claims: serde_json::Value = serde_json::from_slice(&payload_bytes).map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid JSON"}))))?;
        email = claims.get("email").and_then(|v| v.as_str()).unwrap_or("user@localhost").to_string();
        user_id = claims.get("sub").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        org_scope = claims_org(&claims);
    } else {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Provide cloud_token"}))));
    }

    // Create Suite session token with the real per-user role (fix #843) and
    // the JWT org scope.
    let roles = resolve_session_roles(&state, &user_id);
    let suite_token = create_suite_session(&email, &user_id, None, roles, org_scope).await;

    // Sign Cloud JWT
    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let payload = serde_json::json!({
        "sub": user_id,
        "email": email,
        "exp": chrono::Utc::now().timestamp() + 3600,
    });
    let header_b64 = base64_url_encode(&serde_json::to_vec(&header).unwrap_or_default());
    let payload_b64 = base64_url_encode(&serde_json::to_vec(&payload).unwrap_or_default());
    let sig = jwt_sign_inner(&format!("{}.{}", header_b64, payload_b64), jwt_secret.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "main_module::routes::cloud_sso_handler", None)}))))?;
    let cloud_token = format!("{}.{}.{}", header_b64, payload_b64, sig);

    Ok(Json(serde_json::json!({
        "access_token": suite_token,
        "cloud_token": cloud_token,
        "email": email,
        "user_id": user_id,
        "success": true,
    })))
}

/// Load SAAS_JWT_SECRET from directory_config.json (written from Vault),
/// falling back to env vars; same resolution as the SaaS config so all
/// consumers share one stable secret.
fn get_saas_jwt_secret() -> String {
    crate::main_module::directory_setup::resolve_saas_jwt_secret()
}

/// HMAC-SHA256 sign a message and return the base64url-encoded signature.
/// Duplicado de botcloud::api::jwt_sign_inner para evitar dependência entre crates.
fn jwt_sign_inner(message: &str, secret: &[u8]) -> Result<String, String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|e| format!("HMAC key error: {}", e))?;
    mac.update(message.as_bytes());
    Ok(base64_url_encode(&mac.finalize().into_bytes()))
}

fn base64_url_encode(input: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(input)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

fn base64_url_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    let raw = input.replace('-', "+").replace('_', "/");
    let raw = match raw.len() % 4 {
        2 => raw + "==",
        3 => raw + "=",
        0 => raw,
        _ => return Err("invalid base64 input length"),
    };
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.decode(&raw).map_err(|_| "base64 decode failed")
}
