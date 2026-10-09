use super::*;

impl SafeErrorResponse {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            code: None,
            request_id: None,
            details: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    pub fn internal_error() -> Self {
        Self::new("internal_error", "An internal error occurred")
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("bad_request", message)
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::new("not_found", format!("{} not found", resource.into()))
    }

    pub fn unauthorized() -> Self {
        Self::new("unauthorized", "Authentication required")
    }

    pub fn forbidden() -> Self {
        Self::new("forbidden", "You don't have permission to access this resource")
    }

    pub fn rate_limited(retry_after: Option<u64>) -> Self {
        let mut response = Self::new("rate_limited", "Too many requests, please try again later");
        if let Some(secs) = retry_after {
            response = response.with_detail("retry_after_seconds", secs.to_string());
        }
        response
    }

    pub fn validation_error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new("validation_error", message).with_detail("field", field)
    }

    pub fn service_unavailable() -> Self {
        Self::new("service_unavailable", "Service temporarily unavailable")
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("conflict", message)
    }

    pub fn gone(message: impl Into<String>) -> Self {
        Self::new("gone", message)
    }

    pub fn payload_too_large(max_size: Option<u64>) -> Self {
        let mut response = Self::new("payload_too_large", "Request payload is too large");
        if let Some(size) = max_size {
            response = response.with_detail("max_size_bytes", size.to_string());
        }
        response
    }

    pub fn unsupported_media_type(supported: &[&str]) -> Self {
        Self::new(
            "unsupported_media_type",
            "The media type is not supported",
        )
        .with_detail("supported_types", supported.join(", "))
    }

    pub fn method_not_allowed(allowed: &[&str]) -> Self {
        Self::new("method_not_allowed", "HTTP method not allowed for this endpoint")
            .with_detail("allowed_methods", allowed.join(", "))
    }

    pub fn timeout() -> Self {
        Self::new("timeout", "The request timed out")
    }
}
impl IntoResponse for SafeErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.error.as_str() {
            "bad_request" | "validation_error" => StatusCode::BAD_REQUEST,
            "unauthorized" => StatusCode::UNAUTHORIZED,
            "forbidden" => StatusCode::FORBIDDEN,
            "not_found" => StatusCode::NOT_FOUND,
            "method_not_allowed" => StatusCode::METHOD_NOT_ALLOWED,
            "conflict" => StatusCode::CONFLICT,
            "gone" => StatusCode::GONE,
            "payload_too_large" => StatusCode::PAYLOAD_TOO_LARGE,
            "unsupported_media_type" => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
            "timeout" => StatusCode::GATEWAY_TIMEOUT,
            "service_unavailable" => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}
