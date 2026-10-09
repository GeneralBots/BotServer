use super::*;

impl ForbiddenResponse {
    pub(super) fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}
impl IntoResponse for ForbiddenResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "forbidden",
            "message": self.message,
            "code": "FORBIDDEN"
        });

        (StatusCode::FORBIDDEN, Json(body)).into_response()
    }
}
