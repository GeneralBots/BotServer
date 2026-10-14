//! Shared HTTP state and tenant resolution for the fraud endpoints.

use crate::engine::FraudEngine;
use axum::http::HeaderMap;
use base64::Engine as _;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

#[derive(Clone)]
pub struct FraudState {
    pub pool: DbPool,
    pub engine: FraudEngine,
}

impl FraudState {
    pub fn new(pool: DbPool) -> Self {
        let engine = FraudEngine::new(pool.clone());
        Self { pool, engine }
    }
}

/// Resolves the caller's tenant branch from the JWT/session claims, never from
/// client-supplied query params (issue #734). Falls back to the global nil
/// branch for anonymous/internal callers so system-wide operations still work.
pub fn resolve_branch(headers: &HeaderMap) -> Uuid {
    let header = headers
        .get("authorization")
        .or_else(|| headers.get("Authorization"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")));

    let Some(header) = header else {
        return Uuid::nil();
    };
    let Some(payload) = header.split('.').nth(1) else {
        return Uuid::nil();
    };
    // JWT payloads are base64url without padding, but some issuers emit trailing
    // padding; strip it so URL_SAFE_NO_PAD can always decode.
    let unpadded: String = payload.trim_end_matches('=').to_string();
    let Ok(b64) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(unpadded) else {
        return Uuid::nil();
    };
    let Ok(claims) = serde_json::from_slice::<serde_json::Value>(&b64) else {
        return Uuid::nil();
    };

    claims
        .get("branch_id")
        .or_else(|| claims.get("org_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(Uuid::nil())
}
