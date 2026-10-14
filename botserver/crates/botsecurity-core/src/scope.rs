//! Shared branch-scope resolution for tenant-scoped write handlers (#1347).
//!
//! This module replaces the nine per-crate `scope.rs` copies (billing,
//! attendant, marketing, people, products, tickets carried one identical
//! variant; contacts and workspaces carried a second variant that also
//! tolerated multi-token Authorization headers). One implementation now
//! guards every tenant boundary: scope is the most security-relevant helper
//! in the tree, and a copy that drifts is the easiest way to get an
//! authorization rule wrong.
//!
//! Data is owned by the branch (workspace) which is owned by the org (the
//! `.gborg` tenant). When an authenticated request carries a JWT whose email
//! maps to a CRM contact, the branch is derived from that contact so writes
//! land in the caller's own workspace — never in an arbitrary "default" bot
//! branch (issue #730). When no JWT/email is present, callers fall back to
//! their crate-level default context.

use axum::http::HeaderMap;
use diesel::prelude::*;
use uuid::Uuid;

/// Minimal URL-safe base64 decoder (JWT payloads). Returns `None` on invalid
/// input so a malformed header never aborts a request.
fn base64url_decode(input: &str) -> Option<Vec<u8>> {
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

/// Extracts the `email` claim from a Bearer JWT Authorization header.
pub fn email_from_jwt(headers: &HeaderMap) -> Option<String> {
    let token = bearer_token(headers)?;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64url_decode(parts[1])?;
    let json: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    json.get("email").and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extracts the user email from an opaque suite session token (`gb_*`).
/// The suite UI authenticates with a random opaque token that the auth
/// middleware resolves via the session cache (populated at login); the
/// cached entry carries the real user email used to scope CRM data.
pub fn email_from_session(headers: &HeaderMap) -> Option<String> {
    let token = bearer_token(headers)?;
    if token.contains('.') {
        return None;
    }
    crate::lookup_session_cache(token).map(|u| u.email)
}

/// Resolves a suite user's email from the `X-User-ID` header used by the
/// chat/WhatsApp loopback executor (`api.exec`). The loopback hop carries no
/// Authorization header, so the account is identified by id only; its email
/// comes from the `users` table.
pub fn email_from_user_id(headers: &HeaderMap, conn: &mut diesel::PgConnection) -> Option<String> {
    use diesel::sql_types::{Text, Uuid as SqlUuid};
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = Text)]
        email: String,
    }
    let uid = headers.get("x-user-id")?.to_str().ok()?;
    let uid = uuid::Uuid::parse_str(uid).ok()?;
    diesel::sql_query("SELECT email FROM users WHERE id = $1 LIMIT 1")
        .bind::<SqlUuid, _>(uid)
        .get_result::<Row>(conn)
        .optional()
        .ok()
        .flatten()
        .map(|r| r.email)
}

/// Extracts the server-minted `branch_id` JWT claim. Minted at login from
/// the user's verified tenant binding (issue #736), it is the authoritative
/// branch scope; never derived from client input.
pub fn branch_from_claim(headers: &HeaderMap) -> Option<Uuid> {
    let token = bearer_token(headers)?;
    if !token.contains('.') {
        return None;
    }
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64url_decode(parts[1])?;
    let json: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    json.get("branch_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// The Bearer token carried by the `Authorization` header.
///
/// Some proxies append a second comma-separated credential to the header, so
/// the value is trimmed and only the first entry is used (behaviour shared by
/// the contacts/workspaces variants; strictly more permissive than the
/// single-crate copies and safe for both).
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    auth.strip_prefix("Bearer ")
        .map(|t| t.split(',').next().unwrap_or(t).trim())
}

/// Resolves the branch id for the authenticated user. The server-minted
/// `branch_id` claim wins when present (issue #736); otherwise the JWT email
/// (or the session-cache email for opaque suite tokens, or the `X-User-ID`
/// email for the chat loopback) is looked up in `crm_contacts` to derive the
/// contact's `branch_id`.
pub fn branch_from_jwt(
    headers: &HeaderMap,
    conn: &mut diesel::PgConnection,
) -> Option<Uuid> {
    if let Some(claim) = branch_from_claim(headers) {
        return Some(claim);
    }
    let email = email_from_jwt(headers)
        .or_else(|| email_from_session(headers))
        .or_else(|| email_from_user_id(headers, conn))?;
    let contact_branch = {
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            branch_id: Uuid,
        }
        diesel::sql_query(
            "SELECT branch_id FROM crm_contacts WHERE email = $1 LIMIT 1",
        )
        .bind::<diesel::sql_types::Text, _>(&email)
        .get_result::<Row>(conn)
        .optional()
        .ok()
        .flatten()
        .map(|r| r.branch_id)
    };
    // Fall back to the user→org→branch binding when no CRM contact owns the
    // email (issue #808: prod admins without a crm_contacts row were scoped
    // to the nil branch → empty grids).
    contact_branch.or_else(|| crate::tenant::branch_from_user_binding(conn, &email))
}

/// Resolves the branch for the caller using a connection from the pool,
/// for handlers that hold a `DbPool` instead of a live connection.
pub fn branch_from_jwt_pool(
    headers: &HeaderMap,
    pool: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
) -> Option<Uuid> {
    let mut conn = pool.get().ok()?;
    branch_from_jwt(headers, &mut conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_with(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", value.parse().expect("valid header value"));
        headers
    }

    #[test]
    fn email_from_jwt_reads_the_payload_claim() {
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"email":"admin@pragmatismo.com.br"}"#);
        let token = format!("aaa.{payload}.bbb");
        let headers = header_with(&format!("Bearer {token}"));
        assert_eq!(
            email_from_jwt(&headers).as_deref(),
            Some("admin@pragmatismo.com.br")
        );
    }

    #[test]
    fn email_from_jwt_tolerates_a_comma_separated_header() {
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"email":"user@example.com"}"#);
        let token = format!("aaa.{payload}.bbb");
        let headers = header_with(&format!("Bearer {token},extra-credential"));
        assert_eq!(email_from_jwt(&headers).as_deref(), Some("user@example.com"));
    }

    #[test]
    fn email_from_jwt_rejects_malformed_tokens() {
        assert!(email_from_jwt(&header_with("Bearer not-a-jwt")).is_none());
        assert!(email_from_jwt(&header_with("Bearer a.@@@.c")).is_none());
        assert!(email_from_jwt(&header_with("Basic dXNlcjpwYXNz")).is_none());
    }

    #[test]
    fn branch_from_claim_reads_the_server_minted_branch() {
        use base64::Engine;
        let branch = Uuid::new_v4();
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(format!(r#"{{"branch_id":"{branch}"}}"#));
        let token = format!("aaa.{payload}.bbb");
        let headers = header_with(&format!("Bearer {token}"));
        assert_eq!(branch_from_claim(&headers), Some(branch));
    }
}
