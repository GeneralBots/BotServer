//! Tenant-scope resolution for multi-tenant data isolation (issue #734).
//!
//! All reads and writes of branch-scoped tables MUST be constrained to the
//! caller's tenant. The tenant is resolved exclusively from the server-minted
//! JWT claims (`branch_id` is the workspace branch, `org_id` the `.gborg`
//! tenant), never from client-supplied params or headers.
//!
//! Handlers receive a `HeaderMap` and use one of the `*_from_claims` helpers
//! to obtain the authoritative tenant, then apply `branch_id = ?` in queries.

use axum::http::HeaderMap;
use base64::Engine as _;
use uuid::Uuid;

/// Minimal URL-safe base64 decoder (JWT payload decoding).
/// Returns `None` on invalid input so a malformed header never aborts a
/// request or panics in a production path (AGENTS.md).
pub fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
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

/// Extracts the caller's token from an `Authorization` header value.
///
/// `Bearer <token>` is the native form. `Basic <base64>` is accepted as well
/// because native clients (calendar, mail) authenticate with HTTP Basic and
/// have no field for a bearer token: the password field is expected to carry a
/// GeneralBots token, and the username is ignored because the token already
/// identifies the caller. The returned token is still verified on the normal
/// authentication path, so this widens the transport, not the trust.
pub fn token_from_authorization(auth: &str) -> Option<String> {
    if let Some(token) = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
    {
        let token = token.trim();
        return if token.is_empty() {
            None
        } else {
            Some(token.to_string())
        };
    }

    const BASIC_PREFIX_LEN: usize = "basic ".len();
    // `get` keeps the split on a character boundary, so a non-ASCII header is
    // refused instead of panicking.
    let head = auth.get(..BASIC_PREFIX_LEN)?;
    if !head.eq_ignore_ascii_case("basic ") {
        return None;
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(auth[BASIC_PREFIX_LEN..].trim())
        .ok()?;
    let credentials = String::from_utf8(decoded).ok()?;
    let (_, token) = credentials.split_once(':')?;
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

/// Extracts the JWT payload object (middle segment) from the Authorization
/// header, for either the `Bearer` or the `Basic` transport. Invalid tokens
/// yield `None`.
fn jwt_payload(headers: &HeaderMap) -> Option<serde_json::Value> {
    let auth = headers
        .get("authorization")
        .or_else(|| headers.get("Authorization"))?
        .to_str()
        .ok()?;
    let token = token_from_authorization(auth)?;
    if !token.contains('.') {
        return None;
    }
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64url_decode(parts[1].trim_end_matches('='))?;
    serde_json::from_slice(&payload).ok()
}

/// Returns the server-minted `branch_id` claim, the authoritative tenant
/// workspace branch. Never derived from client input.
pub fn branch_from_claims(headers: &HeaderMap) -> Option<Uuid> {
    jwt_payload(headers)?
        .get("branch_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Returns the owning tenant organization id from the JWT claims.
pub fn org_from_claims(headers: &HeaderMap) -> Option<Uuid> {
    let payload = jwt_payload(headers)?;
    payload
        .get("org_id")
        .or_else(|| payload.get("organization_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Returns the `sub` (user id) from the JWT claims.
pub fn user_id_from_claims(headers: &HeaderMap) -> Option<String> {
    jwt_payload(headers)?
        .get("sub")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Returns the `sub` claim from a raw token string. Needed by transports
/// that cannot set headers (SSE via EventSource passes `?token=`).
pub fn user_id_from_claims_subject(token: &str) -> Option<String> {
    let token = token.strip_prefix("Bearer ").unwrap_or(token);
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64url_decode(parts[1].trim_end_matches('='))?;
    serde_json::from_slice::<serde_json::Value>(&payload)
        .ok()?
        .get("sub")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Returns the email claim from the JWT, if present.
pub fn email_from_claims(headers: &HeaderMap) -> Option<String> {
    jwt_payload(headers)?
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Resolves the user's workspace branch from their org membership binding
/// (users → user_organizations → branches) when no server-minted claim and no
/// CRM contact row are available. Returns `None` when the user has no verified
/// binding — callers then fall back to their crate-level default context.
pub fn branch_from_user_binding(
    conn: &mut diesel::PgConnection,
    email: &str,
) -> Option<Uuid> {
    use diesel::prelude::*;
    #[derive(diesel::QueryableByName)]
    struct UserRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }
    let user_id = diesel::sql_query("SELECT id FROM users WHERE email = $1 LIMIT 1")
        .bind::<diesel::sql_types::Text, _>(email)
        .get_result::<UserRow>(conn)
        .optional()
        .ok()
        .flatten()?
        .id;

    #[derive(diesel::QueryableByName)]
    struct BindingRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        org_id: Uuid,
    }
    let org_id = diesel::sql_query(
        "SELECT org_id FROM user_organizations WHERE user_id = $1 ORDER BY is_default DESC, joined_at ASC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .get_result::<BindingRow>(conn)
    .optional()
    .ok()
    .flatten()?
    .org_id;

    #[derive(diesel::QueryableByName)]
    struct BranchRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }
    diesel::sql_query(
        "SELECT id FROM branches WHERE org_id = $1 AND is_active = true ORDER BY created_at ASC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(org_id)
    .get_result::<BranchRow>(conn)
    .optional()
    .ok()
    .flatten()
    .map(|r| r.id)
}
