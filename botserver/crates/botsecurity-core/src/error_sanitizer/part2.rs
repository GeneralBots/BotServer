use super::*;

impl Default for ErrorSanitizer {
    fn default() -> Self {
        Self {
            hide_internal_errors: true,
            log_internal_errors: true,
            include_request_id: true,
            sensitive_patterns: vec![
                "password".to_string(),
                "secret".to_string(),
                "token".to_string(),
                "api_key".to_string(),
                "apikey".to_string(),
                "authorization".to_string(),
                "credential".to_string(),
                "private".to_string(),
                "key".to_string(),
                "database".to_string(),
                "connection".to_string(),
                "dsn".to_string(),
                "postgres".to_string(),
                "mysql".to_string(),
                "redis".to_string(),
                "mongodb".to_string(),
                "aws".to_string(),
                "azure".to_string(),
                "gcp".to_string(),
                "/home/".to_string(),
                "/root/".to_string(),
                "/etc/".to_string(),
                "/var/".to_string(),
                "c:\\".to_string(),
                "d:\\".to_string(),
            ],
        }
    }
}
impl ErrorSanitizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn production() -> Self {
        Self {
            hide_internal_errors: true,
            log_internal_errors: true,
            include_request_id: true,
            sensitive_patterns: Self::default().sensitive_patterns,
        }
    }

    pub fn development() -> Self {
        Self {
            hide_internal_errors: false,
            log_internal_errors: true,
            include_request_id: true,
            sensitive_patterns: Self::default().sensitive_patterns,
        }
    }

    pub fn with_hide_internal(mut self, hide: bool) -> Self {
        self.hide_internal_errors = hide;
        self
    }

    pub fn with_logging(mut self, log: bool) -> Self {
        self.log_internal_errors = log;
        self
    }

    pub fn with_request_id(mut self, include: bool) -> Self {
        self.include_request_id = include;
        self
    }

    pub fn add_sensitive_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.sensitive_patterns.push(pattern.into());
        self
    }

    pub fn sanitize_error<E: std::error::Error>(
        &self,
        error: &E,
        request_id: Option<&str>,
    ) -> SafeErrorResponse {
        let error_string = error.to_string();

        if self.log_internal_errors {
            error!(
                request_id = ?request_id,
                error = %error_string,
                "Internal error occurred"
            );
        }

        if self.hide_internal_errors || self.contains_sensitive(&error_string) {
            let mut response = SafeErrorResponse::internal_error();
            if self.include_request_id {
                if let Some(rid) = request_id {
                    response = response.with_request_id(rid);
                }
            }
            response
        } else {
            let sanitized = self.sanitize_message(&error_string);
            let mut response = SafeErrorResponse::new("error", sanitized);
            if self.include_request_id {
                if let Some(rid) = request_id {
                    response = response.with_request_id(rid);
                }
            }
            response
        }
    }

    pub fn sanitize_message(&self, message: &str) -> String {
        let mut result = message.to_string();

        for pattern in &self.sensitive_patterns {
            if result.to_lowercase().contains(&pattern.to_lowercase()) {
                result = redact_around_pattern(&result, pattern);
            }
        }

        result = redact_stack_traces(&result);
        result = redact_file_paths(&result);
        result = redact_ip_addresses(&result);
        result = redact_connection_strings(&result);

        result
    }

    pub fn contains_sensitive(&self, message: &str) -> bool {
        let lower = message.to_lowercase();
        for pattern in &self.sensitive_patterns {
            if lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }

        if looks_like_stack_trace(message) {
            return true;
        }

        if looks_like_connection_string(message) {
            return true;
        }

        false
    }

    pub fn safe_response_for_status(
        &self,
        status: StatusCode,
        request_id: Option<&str>,
    ) -> SafeErrorResponse {
        let mut response = match status {
            StatusCode::BAD_REQUEST => SafeErrorResponse::bad_request("Invalid request"),
            StatusCode::UNAUTHORIZED => SafeErrorResponse::unauthorized(),
            StatusCode::FORBIDDEN => SafeErrorResponse::forbidden(),
            StatusCode::NOT_FOUND => SafeErrorResponse::not_found("Resource"),
            StatusCode::METHOD_NOT_ALLOWED => SafeErrorResponse::method_not_allowed(&[]),
            StatusCode::CONFLICT => SafeErrorResponse::conflict("Resource conflict"),
            StatusCode::GONE => SafeErrorResponse::gone("Resource no longer available"),
            StatusCode::PAYLOAD_TOO_LARGE => SafeErrorResponse::payload_too_large(None),
            StatusCode::UNSUPPORTED_MEDIA_TYPE => SafeErrorResponse::unsupported_media_type(&[]),
            StatusCode::TOO_MANY_REQUESTS => SafeErrorResponse::rate_limited(None),
            StatusCode::INTERNAL_SERVER_ERROR => SafeErrorResponse::internal_error(),
            StatusCode::SERVICE_UNAVAILABLE => SafeErrorResponse::service_unavailable(),
            StatusCode::GATEWAY_TIMEOUT => SafeErrorResponse::timeout(),
            _ => SafeErrorResponse::new(
                format!("error_{}", status.as_u16()),
                status.canonical_reason().unwrap_or("An error occurred"),
            ),
        };

        if self.include_request_id {
            if let Some(rid) = request_id {
                response = response.with_request_id(rid);
            }
        }

        response
    }
}
/// Redacts from the first occurrence of `pattern` through the end of the token
/// that follows it (`password=abc rest` becomes `[REDACTED] rest`).
///
/// The match is case-insensitive and the cuts are taken on char boundaries of
/// `text` itself. The search deliberately does not lower-case a copy and reuse
/// its offsets: `str::to_lowercase` can change the byte length (`İ` becomes two
/// code points), which made `&text[..start]` panic with "byte index N is not a
/// char boundary" — from inside the sanitizer every error path depends on.
///
/// An empty pattern is ignored; it used to redact the first token of the text.
pub(super) fn redact_around_pattern(text: &str, pattern: &str) -> String {
    if pattern.is_empty() {
        return text.to_string();
    }

    // Compiled only on a hit: callers already matched the pattern
    // case-insensitively before calling. Hoisting these into a cache is tracked
    // with the crate's other per-call regexes.
    let Ok(re) = regex::Regex::new(&format!("(?i){}", regex::escape(pattern))) else {
        return text.to_string();
    };

    let Some(found) = re.find(text) else {
        return text.to_string();
    };

    // `Regex::find` reports char-boundary offsets into `text`, and `token_end`
    // walks chars, so both slices below are always valid.
    let (before, _) = text.split_at(found.start());
    let (_, after) = text.split_at(token_end(text, found.end()));
    format!("{before}[REDACTED]{after}")
}

/// The byte offset just past the token starting at `from`: every character up
/// to the next whitespace, comma or semicolon, or the end of `text`.
fn token_end(text: &str, from: usize) -> usize {
    let Some(rest) = text.get(from..) else {
        return text.len();
    };

    for (offset, ch) in rest.char_indices() {
        if ch.is_whitespace() || ch == ',' || ch == ';' {
            return from + offset;
        }
    }

    text.len()
}
pub(super) fn redact_stack_traces(text: &str) -> String {
    let patterns = [
        r"at .+:\d+:\d+",
        r"File .+, line \d+",
        r"\s+at .+\(.+\)",
        r"\.rs:\d+",
        r"\.go:\d+",
        r"\.py:\d+",
        r"\.java:\d+",
        r"\.js:\d+",
        r"\.ts:\d+",
    ];

    let mut result = text.to_string();
    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, "[STACK_TRACE_REDACTED]").to_string();
        }
    }
    result
}
pub(super) fn redact_file_paths(text: &str) -> String {
    let patterns = [
        r"/[a-zA-Z0-9_\-./]+\.(rs|go|py|java|js|ts|rb|php|c|cpp|h)",
        r"[A-Z]:\\[a-zA-Z0-9_\-\\]+\.\w+",
        r"/home/[a-zA-Z0-9_\-/]+",
        r"/root/[a-zA-Z0-9_\-/]+",
        r"/var/[a-zA-Z0-9_\-/]+",
        r"/etc/[a-zA-Z0-9_\-/]+",
    ];

    let mut result = text.to_string();
    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, "[PATH_REDACTED]").to_string();
        }
    }
    result
}
pub(super) fn redact_ip_addresses(text: &str) -> String {
    let ip_pattern = r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b";
    if let Ok(re) = regex::Regex::new(ip_pattern) {
        re.replace_all(text, "[IP_REDACTED]").to_string()
    } else {
        text.to_string()
    }
}
pub(super) fn redact_connection_strings(text: &str) -> String {
    let patterns = [
        r"postgres://[^\s]+",
        r"postgresql://[^\s]+",
        r"mysql://[^\s]+",
        r"mongodb://[^\s]+",
        r"mongodb\+srv://[^\s]+",
        r"redis://[^\s]+",
        r"amqp://[^\s]+",
        r"jdbc:[^\s]+",
    ];

    let mut result = text.to_string();
    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, "[CONNECTION_STRING_REDACTED]").to_string();
        }
    }
    result
}
pub(super) fn looks_like_stack_trace(text: &str) -> bool {
    let indicators = [
        "at line",
        "stack trace",
        "backtrace",
        "panic:",
        "Traceback",
        "Exception in thread",
        "    at ",
        "Caused by:",
    ];

    let lower = text.to_lowercase();
    for indicator in &indicators {
        if lower.contains(&indicator.to_lowercase()) {
            return true;
        }
    }

    false
}
