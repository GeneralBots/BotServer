use crate::models::{ConnectBody, ItemRow};
use crate::{acl, audit, registry, schema, sync};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use botcore::shared::state::AppState;
use diesel::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

const DEFAULT_SEARCH_LIMIT: i64 = 25;
const MAX_SEARCH_LIMIT: i64 = 100;

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Local base64url decoder (standard and URL alphabets, padded or not).
fn b64url_decode(input: &str) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut out = Vec::new();
    for ch in input.bytes() {
        let value = match ch {
            b'A'..=b'Z' => (ch - b'A') as u32,
            b'a'..=b'z' => (ch - b'a' + 26) as u32,
            b'0'..=b'9' => (ch - b'0' + 52) as u32,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => return None,
        };
        acc = (acc << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

fn jwt_claims(headers: &HeaderMap) -> Option<Value> {
    let token = bearer_token(headers)?;
    let payload = token.split('.').nth(1)?;
    let decoded = b64url_decode(payload)?;
    serde_json::from_slice(&decoded).ok()
}

fn claims_role_is_admin(claims: &Value) -> bool {
    if claims
        .get("role")
        .and_then(Value::as_str)
        .map(|r| r.eq_ignore_ascii_case("admin"))
        .unwrap_or(false)
    {
        return true;
    }
    claims
        .get("roles")
        .and_then(Value::as_array)
        .map(|roles| {
            roles.iter().any(|r| {
                r.as_str().map(|s| s.eq_ignore_ascii_case("admin")).unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn jwt_is_admin(headers: &HeaderMap) -> bool {
    jwt_claims(headers).map(|c| claims_role_is_admin(&c)).unwrap_or(false)
}

fn require_admin(headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    if jwt_is_admin(headers) {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "Administrator access required".to_string()))
    }
}

fn jwt_user_id(headers: &HeaderMap) -> Option<Uuid> {
    let claims = jwt_claims(headers)?;
    for key in ["user_id", "sub"] {
        if let Some(raw) = claims.get(key).and_then(Value::as_str) {
            if let Ok(id) = Uuid::parse_str(raw) {
                return Some(id);
            }
        }
    }
    None
}

fn jwt_org_id(headers: &HeaderMap) -> Option<Uuid> {
    let claims = jwt_claims(headers)?;
    claims.get("org_id").and_then(Value::as_str).and_then(|raw| Uuid::parse_str(raw).ok())
}

fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn db_conn(
    state: &Arc<AppState>,
) -> Result<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>, (StatusCode, String)>
{
    state.conn.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB pool: {e}")))
}

async fn list_connections(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&headers)?;
    let mut conn = db_conn(&state)?;
    use schema::connector_connections::dsl::*;

    let rows: Vec<crate::models::ConnectionRow> = connector_connections
        .order(created_at.desc())
        .limit(200)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query connections: {e}")))?;

    let connections: Vec<Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.id, "org_id": row.org_id, "kind": row.kind,
                "display_name": row.display_name, "status": row.status,
                "last_sync_at": row.last_sync_at,
                "has_credentials": !row.vault_token_ref.is_empty(),
                "created_at": row.created_at, "updated_at": row.updated_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "connections": connections })))
}

async fn create_connection(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ConnectBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&headers)?;
    if registry::connector_for_kind(body.kind.trim()).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported connector kind; available: {:?}", registry::registered_kinds()),
        ));
    }
    let org = jwt_org_id(&headers).ok_or_else(|| {
        (StatusCode::BAD_REQUEST, "Organization claim required".to_string())
    })?;
    if body.credentials.as_object().is_none() {
        return Err((StatusCode::BAD_REQUEST, "Credentials must be a JSON object".to_string()));
    }

    let connection_id = Uuid::new_v4();
    let vault_ref = format!("secret/gbo/connectors/{org}/{connection_id}");
    registry::store_credentials(&vault_ref, &body.credentials).await.map_err(|e| {
        tracing::error!("botconnectors: credential store failed on create: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to persist credentials".to_string())
    })?;

    let mut conn = db_conn(&state)?;
    use schema::connector_connections::dsl::*;
    diesel::insert_into(connector_connections)
        .values((
            id.eq(connection_id),
            org_id.eq(org),
            kind.eq(body.kind.trim()),
            display_name.eq(body.display_name.as_deref().filter(|d| !d.trim().is_empty())),
            vault_token_ref.eq(&vault_ref),
            status.eq("connected"),
            cursors.eq(serde_json::json!({})),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Create connection: {e}")))?;

    tracing::info!(target: "botconnectors", connection_id = %connection_id, kind = %body.kind, org_id = %org, "connector connection created");
    Ok(Json(serde_json::json!({
        "status": "created", "id": connection_id, "kind": body.kind.trim(),
    })))
}

async fn sync_connection_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(connection_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&headers)?;
    let outcome = sync::sync_connection(&state, connection_id).await?;
    Ok(Json(serde_json::to_value(outcome)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Outcome encode failed".to_string()))?))
}

async fn delete_connection(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(connection_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&headers)?;
    let mut conn = db_conn(&state)?;

    let purged = diesel::sql_query("DELETE FROM indexed_items WHERE connection_id = $1")
        .bind::<diesel::sql_types::Uuid, _>(connection_id)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Purge items: {e}")))?;

    let updated =
        diesel::sql_query(
            "UPDATE connector_connections SET status = 'disconnected', updated_at = NOW() WHERE id = $1",
        )
        .bind::<diesel::sql_types::Uuid, _>(connection_id)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Disconnect: {e}")))?;

    if updated == 0 {
        return Err((StatusCode::NOT_FOUND, format!("Connector connection {connection_id} not found")));
    }

    if let Some(row) = load_vault_ref(&mut conn, connection_id) {
        registry::delete_credentials(&row).await;
    }

    Ok(Json(serde_json::json!({
        "status": "disconnected", "id": connection_id, "items_purged": purged as i64,
    })))
}

fn load_vault_ref(conn: &mut PgConnection, connection_id: Uuid) -> Option<String> {
    #[derive(diesel::QueryableByName)]
    struct RefRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        vault_token_ref: String,
    }
    diesel::sql_query("SELECT vault_token_ref FROM connector_connections WHERE id = $1")
        .bind::<diesel::sql_types::Uuid, _>(connection_id)
        .get_result::<RefRow>(conn)
        .optional()
        .ok()
        .flatten()
        .map(|r| r.vault_token_ref)
}

async fn search_connectors(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&headers)?;
    let user_id = jwt_user_id(&headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Valid JWT required".to_string()))?;

    let q = params.get("q").map(|s| s.trim().to_string()).unwrap_or_default();
    if q.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Query parameter 'q' is required".to_string()));
    }
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .map(|l| l.clamp(1, MAX_SEARCH_LIMIT))
        .unwrap_or(DEFAULT_SEARCH_LIMIT);
    let sources: Option<Vec<String>> = match params.get("sources").map(|s| s.trim()) {
        Some(raw) if !raw.is_empty() => {
            Some(raw.split(',').map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect())
        }
        _ => None,
    };

    let mut conn = db_conn(&state)?;
    let items: Vec<ItemRow> =
        acl::search_visible(&mut conn, user_id, &q, sources.clone(), limit)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Search failed: {e}")))?;

    let sources_hit: Vec<String> = sources.unwrap_or_else(|| {
        registry::registered_kinds().into_iter().map(|k| k.to_string()).collect()
    });
    audit::audit_query(&mut conn, Some(user_id), &sha256_hex(&q), &sources_hit, items.len());

    Ok(Json(serde_json::json!({
        "query": { "q_hash": sha256_hex(&q), "sources": sources_hit, "limit": limit },
        "count": items.len(),
        "items": items,
    })))
}

pub fn configure() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/connectors", get(list_connections).post(create_connection))
        .route("/api/connectors/:id/sync", post(sync_connection_route))
        .route("/api/connectors/:id", delete(delete_connection))
        .route("/api/connectors/search", get(search_connectors))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_with_payload(payload: &str) -> HeaderMap {
        fn b64url_encode(bytes: &[u8]) -> String {
            const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
            let mut out = String::new();
            let mut acc: u32 = 0;
            let mut bits: u32 = 0;
            for byte in bytes {
                acc = (acc << 8) | *byte as u32;
                bits += 8;
                while bits >= 6 {
                    bits -= 6;
                    out.push(ALPHABET[((acc >> bits) & 0x3F) as usize] as char);
                }
            }
            if bits > 0 {
                out.push(ALPHABET[((acc << (6 - bits)) & 0x3F) as usize] as char);
            }
            out
        }
        let token =
            format!("eyJhbGciOiJIUzI1NiJ9.{}.c2ln", b64url_encode(payload.as_bytes()));
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            axum::http::HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers
    }

    #[test]
    fn decodes_padded_and_unpadded_b64url() {
        assert_eq!(b64url_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(b64url_decode("aGVsbG8").unwrap(), b"hello");
        assert_eq!(b64url_decode("").unwrap(), b"");
        assert!(b64url_decode("*").is_none());
    }

    #[test]
    fn admin_gate_accepts_role_and_rejects_other() {
        let admin = header_with_payload(r#"{"role":"admin","user_id":"00000000-0000-0000-0000-000000000001"}"#);
        assert!(jwt_is_admin(&admin));
        assert_eq!(require_admin(&admin), Ok(()));
        let user = header_with_payload(r#"{"role":"user"}"#);
        assert!(!jwt_is_admin(&user));
        assert!(require_admin(&user).is_err());
        assert!(!jwt_is_admin(&HeaderMap::new()));
    }

    fn jwt_groups(headers: &HeaderMap) -> Vec<String> {
        jwt_claims(headers)
            .and_then(|claims| {
                claims
                    .get("groups")
                    .and_then(|groups| groups.as_array())
                    .map(|groups| {
                        groups
                            .iter()
                            .filter_map(|g| g.as_str().map(str::to_string))
                            .collect()
                    })
            })
            .unwrap_or_default()
    }

    #[test]
    fn extracts_groups_and_org_from_claims() {
        let headers = header_with_payload(
            r#"{"role":"admin","org_id":"00000000-0000-0000-0000-000000000002","groups":["sales","legal"]}"#,
        );
        assert_eq!(
            jwt_org_id(&headers),
            Some(Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap())
        );
        assert_eq!(jwt_groups(&headers), vec!["sales".to_string(), "legal".to_string()]);
        assert_eq!(jwt_user_id(&HeaderMap::new()), None);
    }

    #[test]
    fn hash_is_stable_hex() {
        assert_eq!(sha256_hex("abc").len(), 64);
        assert_eq!(sha256_hex("abc"), sha256_hex("abc"));
        assert_ne!(sha256_hex("abc"), sha256_hex("abd"));
    }
}
