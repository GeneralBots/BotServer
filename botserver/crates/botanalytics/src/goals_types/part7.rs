use super::*;

impl axum::response::IntoResponse for GoalsError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            Self::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg.clone()),
            Self::Unauthorized(msg) => (axum::http::StatusCode::UNAUTHORIZED, msg.clone()),
            Self::Validation(msg) => (axum::http::StatusCode::BAD_REQUEST, msg.clone()),
            Self::Database(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };
        (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
    }
}
