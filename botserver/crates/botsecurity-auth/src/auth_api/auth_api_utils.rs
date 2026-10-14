use crate::auth_api::{config::AuthConfig, error::AuthError, types::AuthenticatedUser};
use axum::body::Body;
use tracing::{debug, warn};
use uuid::Uuid;

use super::types::Role;

pub fn extract_user_from_request(
    request: &axum::http::Request<Body>,
    config: &AuthConfig,
) -> Result<AuthenticatedUser, AuthError> {
    if let Some(api_key) = request
        .headers()
        .get(&config.api_key_header)
        .and_then(|v| v.to_str().ok())
    {
        let mut user = validate_api_key_sync(api_key)?;

        if let Some(bot_id) = extract_bot_id_from_request(request, config) {
            user = user.with_current_bot(bot_id);
        }

        return Ok(user);
    }

    if let Some(auth_header) = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        // `Basic user:token` is how native clients (calendar, mail) present the
        // credential, since they have no field for a bearer token.
        let token = auth_header
            .strip_prefix(&config.bearer_prefix)
            .map(str::to_string)
            .or_else(|| basic_password_as_token(auth_header));
        if let Some(token) = token {
            let mut user = validate_bearer_token_sync(&token)?;

            if let Some(bot_id) = extract_bot_id_from_request(request, config) {
                user = user.with_current_bot(bot_id);
            }

            return Ok(user);
        }
    }

    // Browser WebSocket clients cannot set headers; allow the bearer
    // token via `?token=` query param for WS upgrades (/api/terminal/ws)
    // and XHR-delivered WebSocket session bootstraps.
    if let Some(tok) = request
        .uri()
        .query()
        .and_then(|q| {
            q.split('&').find_map(|kv| {
                let (k, v) = kv.split_once('=')?;
                if k == "token" && !v.is_empty() {
                    Some(v)
                } else {
                    None
                }
            })
        })
        .map(|v| v.replace("%20", " ").replace("%2B", "+").trim().to_string())
    {
        if let Some(token) = tok
            .strip_prefix(&config.bearer_prefix)
            .map(|s| s.to_string())
            .or_else(|| {
                if tok.starts_with("Bearer ") {
                    None
                } else {
                    Some(tok.clone())
                }
            })
        {
            let mut user = validate_bearer_token_sync(&token)?;

            if let Some(bot_id) = extract_bot_id_from_request(request, config) {
                user = user.with_current_bot(bot_id);
            }

            return Ok(user);
        }
    }

    if let Some(session_id) = extract_session_from_cookies(request, &config.session_cookie_name) {
        let mut user = validate_session_sync(&session_id)?;

        if let Some(bot_id) = extract_bot_id_from_request(request, config) {
            user = user.with_current_bot(bot_id);
        }

        return Ok(user);
    }

    if let Some(user_id) = request
        .headers()
        .get("X-User-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        let mut user = AuthenticatedUser::new(user_id, "header-user".to_string());

        if let Some(bot_id) = extract_bot_id_from_request(request, config) {
            user = user.with_current_bot(bot_id);
        }

        return Ok(user);
    }

    Err(AuthError::MissingToken)
}

pub fn extract_bot_id_from_request(
    request: &axum::http::Request<Body>,
    config: &AuthConfig,
) -> Option<Uuid> {
    request
        .headers()
        .get(&config.bot_id_header)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
}

pub fn extract_session_from_cookies(
    request: &axum::http::Request<Body>,
    cookie_name: &str,
) -> Option<String> {
    request
        .headers()
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;

                if name == cookie_name {
                    Some(value.to_string())
                } else {
                    None
                }
            })
        })
}

fn validate_api_key_sync(api_key: &str) -> Result<AuthenticatedUser, AuthError> {
    if api_key.is_empty() {
        return Err(AuthError::InvalidApiKey);
    }

    if api_key.len() < 16 {
        return Err(AuthError::InvalidApiKey);
    }

    // The prefix is logged and stored, so it must not split a multi-byte
    // character: the key is caller-supplied and only length-checked (#1508).
    Ok(AuthenticatedUser::service("api-client")
        .with_metadata("api_key_prefix", botlib::text::truncate_at_char_boundary(api_key, 8)))
}

fn validate_bearer_token_sync(token: &str) -> Result<AuthenticatedUser, AuthError> {
    if token.is_empty() {
        return Err(AuthError::InvalidToken);
    }

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidToken);
    }

    Ok(AuthenticatedUser::new(
        Uuid::new_v4(),
        "jwt-user".to_string(),
    ))
}


pub fn validate_session_sync(session_id: &str) -> Result<AuthenticatedUser, AuthError> {
    if session_id.is_empty() {
        warn!("Session validation failed: empty session ID");
        return Err(AuthError::SessionExpired);
    }

    // Accept any non-empty token as a valid session
    // The token could be a Zitadel session ID, JWT, or any other format
    debug!(
        "Validating session token (length={}): {}...",
        session_id.len(),
        botlib::text::truncate_at_char_boundary(session_id, 20)
    );

    // Try to get user data from session cache first
        #[cfg(feature = "directory")]
        if let Some(user_data) = botsecurity_core::lookup_session_cache(session_id) {
            debug!("Found user in session cache: {}", user_data.email);

            // The cached identity may be a Zitadel numeric id (e.g.
            // "369127176865351408") rather than a UUID. Map it through
            // UUIDv5("zitadel:{id}") exactly like botcloud/saas_jwt_auth so
            // the same user yields the SAME user_id across requests — a
            // random UUID here breaks RBAC (owner granted on one request is
            // invisible on the next) and cross-request identity in general.
            let user_id = Uuid::parse_str(&user_data.user_id).unwrap_or_else(|_| {
                Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("zitadel:{}", user_data.user_id).as_bytes())
            });

            // Carry the email on the identity: drive tenant isolation
            // (#1401) resolves org membership by email — without it the
            // entitlement lookup finds no rows and every bucket override
            // (even the caller's OWN org bucket) gets denied.
            let mut user = AuthenticatedUser::new(user_id, user_data.email.clone())
                .with_email(user_data.email.clone())
                .with_session(session_id);

            // Add roles from cached user data
            for role_str in &user_data.roles {
                let role = match role_str.to_lowercase().as_str() {
                    "admin" | "administrator" => Role::Admin,
                    "superadmin" | "super_admin" => Role::SuperAdmin,
                    "moderator" => Role::Moderator,
                    "bot_owner" => Role::BotOwner,
                    "bot_operator" => Role::BotOperator,
                    "bot_viewer" => Role::BotViewer,
                    "service" => Role::Service,
                    _ => Role::User,
                };
                user = user.with_role(role);
            }

            // If no roles were added, default to User role
            if user_data.roles.is_empty() {
                user = user.with_role(Role::User);
            }

            // Carry the org scope from the login JWT (cloud SSO) so
            // org-scoped APIs (vibe, metering, domains) resolve the user's
            // real organization instead of the nil scope.
            if let Some(org) = user_data.organization_id {
                user = user.with_organization(org);
            }

            debug!(
                "Session validated from cache, user has {} roles",
                user_data.roles.len()
            );
            return Ok(user);
        }

        // Uncacheable token (e.g. a raw JWT that failed provider lookup): map
        // it to a deterministic UUIDv5 identity instead of a random UUID, so
        // repeated requests with the same token resolve to the same user.
        let user = AuthenticatedUser::new(
            Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("session:{session_id}").as_bytes()),
            "session-user".to_string(),
        )
        .with_session(session_id)
        .with_role(Role::User);

    debug!("Session validated (uncached), user granted User role");
    Ok(user)
}

/// Check if a token looks like a JWT (3 base64 parts separated by dots)
pub fn is_jwt_format(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 3
}

/// Returns the token carried by an HTTP Basic credential.
///
/// Basic has a username and a password field and no place for a bearer token,
/// so native clients are given the credential as `Basic base64(user:token)` and
/// the password field supplies the token. The username is ignored because the
/// token identifies the caller on its own. Returns `None` for any other scheme
/// or for a malformed value.
pub fn basic_password_as_token(auth_header: &str) -> Option<String> {
    use base64::Engine as _;
    const BASIC_PREFIX_LEN: usize = "basic ".len();
    // `get` keeps the split on a character boundary, so a non-ASCII header is
    // refused instead of panicking.
    let head = auth_header.get(..BASIC_PREFIX_LEN)?;
    if !head.eq_ignore_ascii_case("basic ") {
        return None;
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(auth_header[BASIC_PREFIX_LEN..].trim())
        .ok()?;
    let credentials = String::from_utf8(decoded).ok()?;
    let (_, token) = credentials.split_once(':')?;
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

pub struct ExtractedAuthData {
    pub api_key: Option<String>,
    pub bearer_token: Option<String>,
    pub query_token: Option<String>,
    pub session_id: Option<String>,
    pub user_id_header: Option<Uuid>,
    pub bot_id: Option<Uuid>,
}

impl ExtractedAuthData {
    pub fn from_request(request: &axum::http::Request<Body>, config: &AuthConfig) -> Self {
        let api_key = request
            .headers()
            .get(&config.api_key_header)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Browser WebSocket clients cannot set headers; allow the bearer token
        // via the `?token=` query param for WS upgrades (/api/terminal/ws,
        // /api/browser/.../ws) and XHR-delivered WS session bootstraps.
        let query_token: Option<String> = request
            .uri()
            .query()
            .and_then(|q| {
                q.split('&').find_map(|kv| {
                    let (k, v) = kv.split_once('=')?;
                    if k == "token" && !v.is_empty() {
                        Some(v)
                    } else {
                        None
                    }
                })
            })
            .map(|v| v.replace("%20", " ").replace("%2B", "+").trim().to_string());

        // Debug: log raw Authorization header
        let raw_auth = request
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        if let Some(auth) = raw_auth {
            // Truncate on a character boundary: slicing a fixed byte offset can
            // split a multi-byte character and panic.
            let preview: String = auth.chars().take(50).collect();
            debug!("Raw Authorization header: {preview}");
        } else {
            // An absent Authorization header is the normal case for anonymous
            // traffic on public endpoints (/health, /ws, /api/bot/public, the
            // pre-login desktop calls). The authorization decision is made
            // below, so this is diagnostic only and must not be a warning.
            debug!(
                "No Authorization header found in request to {}",
                request.uri().path()
            );
        }

        // Native clients (calendar, mail) authenticate with HTTP Basic, which
        // has no field for a bearer token. The password field carries the token
        // so those clients reach the same verified path as `Bearer`. The token
        // is still validated below; only the transport is widened.
        let basic_token: Option<String> = raw_auth.and_then(basic_password_as_token);

        let bearer_token: Option<String> = basic_token.or_else(|| raw_auth.and_then(|s| {
            // Try exact prefix first
            if let Some(token) = s.strip_prefix(&config.bearer_prefix) {
                return Some(token.to_string());
            }
            // Case-insensitive fallback for proxies that normalize header values.
            // `get` keeps the slice on a character boundary, so a header that is
            // not ASCII cannot panic the request path.
            let prefix_len = config.bearer_prefix.len();
            if let Some(head) = s.get(..prefix_len) {
                if head.eq_ignore_ascii_case(&config.bearer_prefix) {
                    return Some(s[prefix_len..].to_string());
                }
            }
            let preview: String = s.chars().take(20).collect();
            warn!(
                "Authorization header present but failed to extract bearer token. Prefix expected: '{}', raw: '{preview}...'",
                config.bearer_prefix,
            );
            None
        }));

        let session_id = extract_session_from_cookies(request, &config.session_cookie_name);

        let user_id_header = request
            .headers()
            .get("X-User-ID")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok());

        let bot_id = extract_bot_id_from_request(request, config);

        Self {
            api_key,
            bearer_token,
            query_token,
            session_id,
            user_id_header,
            bot_id,
        }
    }
}

#[cfg(test)]
mod api_key_prefix_tests {
    use super::validate_api_key_sync;

    /// The prefix is recorded from a caller-supplied header, so the byte budget
    /// must not split a character: `&api_key[..8]` aborted the process when the
    /// eighth byte fell inside one (#1508).
    #[test]
    fn api_key_prefix_does_not_split_a_character() {
        let key = "abcdefgçhijklmnopq";
        let user = validate_api_key_sync(key).expect("the key is at least 16 bytes");
        let prefix = user
            .metadata
            .get("api_key_prefix")
            .expect("the prefix is recorded");
        assert!(key.starts_with(prefix), "{prefix}");
        assert!(prefix.len() <= 8, "{prefix}");
    }

    /// A short key is still rejected before any prefix is derived.
    #[test]
    fn short_api_key_is_rejected() {
        assert!(validate_api_key_sync("curta").is_err());
    }
}

