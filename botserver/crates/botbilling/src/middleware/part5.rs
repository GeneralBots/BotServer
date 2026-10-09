use super::*;

impl IntoResponse for RateLimitExceededResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "rate_limit_exceeded",
            "message": self.message,
            "code": "RATE_LIMIT_EXCEEDED",
            "retry_after": 60
        });

        (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Content-Type", "application/json"),
                ("Retry-After", "60"),
                ("X-RateLimit-Exceeded", "true"),
            ],
            Json(body),
        )
            .into_response()
    }
}
