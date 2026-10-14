pub mod analytics_types;
pub mod handlers;
pub mod handlers_activity;
pub mod handlers_charts;
pub mod handlers_dashboard;
pub mod insights;
pub mod insights_types;
pub mod reports;
pub mod routes;
pub mod schema;

#[cfg(feature = "goals")]
pub mod goals;

#[cfg(feature = "goals")]
pub mod goals_ui;

#[cfg(feature = "goals")]
pub mod goals_types;

use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub type GetDefaultBotFn = Arc<dyn Fn(&mut diesel::PgConnection) -> Uuid + Send + Sync>;

pub type GetBotContextFn = Arc<dyn Fn() -> (Uuid, Uuid) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub username: String,
}

#[axum::async_trait]
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for AuthenticatedUser {
    type Rejection = (axum::http::StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        use base64::Engine;
        use axum::http::header::AUTHORIZATION;

        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or((axum::http::StatusCode::UNAUTHORIZED, "Missing bearer token"))?;

        let payload = header
            .split('.')
            .nth(1)
            .ok_or((axum::http::StatusCode::UNAUTHORIZED, "Invalid token"))?;

        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "Invalid token"))?;

        let claims: serde_json::Value = serde_json::from_slice(&decoded)
            .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "Invalid token"))?;

        let user_id = claims
            .get("sub")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .or_else(|| {
                claims
                    .get("user_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
            })
            .ok_or((axum::http::StatusCode::UNAUTHORIZED, "Invalid user claim"))?;

        let username = claims
            .get("email")
            .or_else(|| claims.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or("user")
            .to_string();

        Ok(Self { user_id, username })
    }
}
