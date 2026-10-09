use super::*;

impl UnauthorizedResponse {
    pub(super) fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}
impl IntoResponse for UnauthorizedResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "unauthorized",
            "message": self.message,
            "code": "UNAUTHORIZED"
        });

        (
            StatusCode::UNAUTHORIZED,
            [
                ("Content-Type", "application/json"),
                ("WWW-Authenticate", "Bearer"),
            ],
            Json(body),
        )
            .into_response()
    }
}
