use super::*;

impl IntoResponse for MessageQuotaExceededResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "message_quota_exceeded",
            "message": self.message,
            "code": "MESSAGE_QUOTA_EXCEEDED",
            "upgrade_url": "/billing/upgrade"
        });

        (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Content-Type", "application/json"),
                ("X-Message-Quota-Exceeded", "true"),
            ],
            Json(body),
        )
            .into_response()
    }
}
