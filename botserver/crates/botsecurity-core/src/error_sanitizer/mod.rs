pub(crate) use axum::{ http::StatusCode, response::{IntoResponse, Response}, Json, };
pub(crate) use serde::Serialize;
pub(crate) use std::collections::HashMap;
pub(crate) use tracing::{error, warn};

#[derive(Debug, Clone, Serialize)]
pub struct SafeErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct ErrorSanitizer {
    hide_internal_errors: bool,
    log_internal_errors: bool,
    include_request_id: bool,
    sensitive_patterns: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::part2::*;

    #[derive(Debug)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn test_safe_error_response_new() {
        let response = SafeErrorResponse::new("test_error", "Test message");
        assert_eq!(response.error, "test_error");
        assert_eq!(response.message, "Test message");
        assert!(response.code.is_none());
        assert!(response.request_id.is_none());
    }

    #[test]
    fn test_safe_error_response_builder() {
        let response = SafeErrorResponse::new("error", "message")
            .with_code("E001")
            .with_request_id("req-123")
            .with_detail("field", "value");

        assert_eq!(response.code, Some("E001".to_string()));
        assert_eq!(response.request_id, Some("req-123".to_string()));
        assert_eq!(
            response.details.as_ref().and_then(|d| d.get("field")),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_factory_methods() {
        let internal = SafeErrorResponse::internal_error();
        assert_eq!(internal.error, "internal_error");

        let not_found = SafeErrorResponse::not_found("User");
        assert_eq!(not_found.error, "not_found");
        assert!(not_found.message.contains("User"));

        let rate_limited = SafeErrorResponse::rate_limited(Some(30));
        assert_eq!(rate_limited.error, "rate_limited");
        assert!(rate_limited.details.is_some());
    }

    #[test]
    fn test_error_sanitizer_hides_sensitive() {
        let sanitizer = ErrorSanitizer::production();
        let error = TestError("Connection failed: password=secret123".to_string());
        let response = sanitizer.sanitize_error(&error, Some("req-123"));

        assert_eq!(response.error, "internal_error");
        assert!(!response.message.contains("secret123"));
    }

    #[test]
    fn test_error_sanitizer_development() {
        let sanitizer = ErrorSanitizer::development();
        let error = TestError("Simple error message".to_string());
        let response = sanitizer.sanitize_error(&error, None);

        assert_eq!(response.message, "Simple error message");
    }

    /// Micro-benchmark for the redaction path (#1493). Ignored by default so it
    /// never slows the suite down; run it explicitly with
    /// `cargo test --lib -p botsecurity-core -- --ignored --nocapture bench_`
    /// and compare the printed per-call cost with and without the pattern
    /// cache.
    #[test]
    #[ignore = "micro-benchmark, run with --ignored --nocapture"]
    fn bench_redaction_per_call() {
        // An error shaped like the ones this path sees: a connection string, a
        // file path, an IP and a stack trace.
        let text = "Connection failed for postgres://app:pw@10.1.2.3:5432/app\n\
                    at /home/app/src/main.rs:42:17\n\
                    Error: connect ECONNREFUSED\n";
        const ITERATIONS: u32 = 5_000;

        let start = std::time::Instant::now();
        for _ in 0..ITERATIONS {
            let _ = part2::redact_stack_traces(text);
            let _ = part2::redact_file_paths(text);
            let _ = part2::redact_ip_addresses(text);
            let _ = part2::redact_connection_strings(text);
        }
        let elapsed = start.elapsed();
        println!(
            "#1493 bench: {ITERATIONS} redactions in {elapsed:?} = {:?} per call",
            elapsed / ITERATIONS
        );
    }

    #[test]
    fn test_contains_sensitive() {
        let sanitizer = ErrorSanitizer::default();

        assert!(sanitizer.contains_sensitive("Failed with password=abc"));
        assert!(sanitizer.contains_sensitive("API_KEY is invalid"));
        assert!(sanitizer.contains_sensitive("at /home/user/app.rs:42"));
        assert!(!sanitizer.contains_sensitive("Simple error"));
    }

    #[test]
    fn test_sanitize_message() {
        let sanitizer = ErrorSanitizer::default();

        let result = sanitizer.sanitize_message("Error at /home/user/file.rs:42");
        assert!(!result.contains("/home/user"));

        let result = sanitizer.sanitize_message("postgres://user:pass@host/db");
        assert!(!result.contains("user:pass"));
    }

    #[test]
    fn test_redact_ip_addresses() {
        let result = redact_ip_addresses("Connection from 192.168.1.100 failed");
        assert!(!result.contains("192.168.1.100"));
        assert!(result.contains("[IP_REDACTED]"));
    }

    #[test]
    fn test_redact_connection_strings() {
        let result = redact_connection_strings("Using postgres://admin:secret@localhost/mydb");
        assert!(!result.contains("admin:secret"));
        assert!(result.contains("[CONNECTION_STRING_REDACTED]"));
    }

    #[test]
    fn test_looks_like_stack_trace() {
        assert!(looks_like_stack_trace("panic: something went wrong"));
        assert!(looks_like_stack_trace("Traceback (most recent call last):"));
        assert!(looks_like_stack_trace("    at com.example.Main.run"));
        assert!(!looks_like_stack_trace("Simple error message"));
    }

    #[test]
    fn test_looks_like_connection_string() {
        assert!(looks_like_connection_string("host=localhost dbname=test user=admin"));
        assert!(looks_like_connection_string("postgres://localhost/db"));
        assert!(!looks_like_connection_string("Simple message"));
    }

    #[test]
    fn test_redact_around_pattern_redacts_the_token_after_the_match() {
        assert_eq!(redact_around_pattern("password=hunter2 rest", "password"), "[REDACTED] rest");
        assert_eq!(redact_around_pattern("value is secret", "secret"), "value is [REDACTED]");
        assert_eq!(redact_around_pattern("api_key=abc,next=1", "api_key"), "[REDACTED],next=1");
        assert_eq!(redact_around_pattern("token", "token"), "[REDACTED]");
        assert_eq!(redact_around_pattern("PASSWORD=x", "password"), "[REDACTED]");
    }

    #[test]
    fn test_redact_around_pattern_leaves_unmatched_text_alone() {
        assert_eq!(redact_around_pattern("nothing here", "zzz"), "nothing here");
        // An empty pattern must not eat the first token of the message.
        assert_eq!(redact_around_pattern("keep me", ""), "keep me");
    }

    #[test]
    fn test_redact_around_pattern_survives_multibyte_before_the_match() {
        // `İ` lowercases to two code points, so the lowercased copy is longer
        // than the input. Reusing its byte offsets produced
        // "İstanbul error p[REDACTED]2 tail" — a partial secret leak plus a
        // misplaced marker.
        let result = redact_around_pattern("İstanbul error password=hunter2 tail", "password");
        assert_eq!(result, "İstanbul error [REDACTED] tail");
    }

    #[test]
    fn test_redact_around_pattern_cuts_multibyte_tokens() {
        // Used to return "[REDACTED]ta fim", leaking the tail of the token.
        let result = redact_around_pattern("senha=ção-secreta fim", "senha");
        assert_eq!(result, "[REDACTED] fim");
    }

    #[test]
    fn test_redact_around_pattern_handles_a_lowercase_that_shrinks() {
        // `K` (Kelvin sign) is three bytes and lowercases to one, so the match
        // offset taken from the lowercased copy landed inside the `€` —
        // "byte index 5 is not a char boundary", aborting the process.
        let result = redact_around_pattern("\u{212A}\u{20AC} password=hunter2 tail", "password");
        assert_eq!(result, "\u{212A}\u{20AC} [REDACTED] tail");
    }

    #[test]
    fn test_safe_response_for_status() {
        let sanitizer = ErrorSanitizer::production();

        let response = sanitizer.safe_response_for_status(StatusCode::NOT_FOUND, Some("req-123"));
        assert_eq!(response.error, "not_found");
        assert_eq!(response.request_id, Some("req-123".to_string()));

        let response = sanitizer.safe_response_for_status(StatusCode::INTERNAL_SERVER_ERROR, None);
        assert_eq!(response.error, "internal_error");
    }

    #[test]
    fn test_sanitize_for_log() {
        let result = sanitize_for_log("password=secret123 at /home/user/app.rs");
        assert!(!result.contains("secret123"));
        assert!(!result.contains("/home/user"));
    }

    #[test]
    fn test_config_builder() {
        let sanitizer = ErrorSanitizer::new()
            .with_hide_internal(false)
            .with_logging(false)
            .with_request_id(false)
            .add_sensitive_pattern("custom_secret");

        assert!(!sanitizer.hide_internal_errors);
        assert!(!sanitizer.log_internal_errors);
        assert!(!sanitizer.include_request_id);
        assert!(sanitizer.sensitive_patterns.contains(&"custom_secret".to_string()));
    }
}

mod part1;
mod part2;
mod part3;

pub use part3::*;
