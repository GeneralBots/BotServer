use super::*;

impl IntoResponse for QuotaExceededResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "quota_exceeded",
            "message": self.message,
            "code": "QUOTA_EXCEEDED"
        });

        (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Content-Type", "application/json"),
                ("X-Quota-Exceeded", "true"),
            ],
            Json(body),
        )
            .into_response()
    }
}
