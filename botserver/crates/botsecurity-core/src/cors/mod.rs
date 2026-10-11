pub(crate) use axum::http::{header, HeaderValue, Method};
pub(crate) use std::collections::HashSet;
pub(crate) use tower_http::cors::{AllowOrigin, CorsLayer};
pub(crate) use tracing::info;

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<Method>,
    pub allowed_headers: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age_secs: u64,
}

static CORS_ALLOWED_ORIGINS: std::sync::RwLock<Option<Vec<String>>> = std::sync::RwLock::new(None);

#[derive(Debug, Clone)]
pub struct OriginValidator {
    allowed_origins: HashSet<String>,
    allow_localhost: bool,
    allowed_patterns: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::part3::*;

    #[test]
    fn test_default_config() {
        let config = CorsConfig::default();
        assert!(config.allowed_origins.is_empty());
        assert!(config.allow_credentials);
        assert_eq!(config.max_age_secs, 3600);
    }

    #[test]
    fn test_production_config() {
        let config = CorsConfig::production();
        assert!(config.allowed_origins.is_empty());
        assert!(config.allow_credentials);
        assert_eq!(config.max_age_secs, 7200);
    }

    #[test]
    fn test_development_config() {
        let config = CorsConfig::development();
        assert!(!config.allowed_origins.is_empty());
        assert!(config.allowed_origins.contains(&"".to_string()));
    }

    #[test]
    fn test_api_config() {
        let config = CorsConfig::api();
        assert!(!config.allow_credentials);
        assert_eq!(config.max_age_secs, 86400);
    }

    #[test]
    fn test_builder_methods() {
        let config = CorsConfig::new()
            .with_origins(vec!["https://example.com".to_string()])
            .with_credentials(false)
            .with_max_age(1800);

        assert_eq!(config.allowed_origins.len(), 1);
        assert!(!config.allow_credentials);
        assert_eq!(config.max_age_secs, 1800);
    }

    #[test]
    fn test_add_origin() {
        let config = CorsConfig::new()
            .add_origin("https://example.com")
            .add_origin("https://api.example.com");

        assert_eq!(config.allowed_origins.len(), 2);
    }

    #[test]
    fn test_add_header() {
        let config = CorsConfig::new().add_header("X-Custom-Header");
        assert!(config.allowed_headers.contains(&"X-Custom-Header".to_string()));
    }

    #[test]
    fn test_valid_origin_format() {
        assert!(is_valid_origin_format("https://example.com"));
        assert!(is_valid_origin_format(""));
        assert!(is_valid_origin_format("https://api.example.com:8443"));

        assert!(!is_valid_origin_format("ftp://example.com"));
        assert!(!is_valid_origin_format("javascript:alert(1)"));
        assert!(!is_valid_origin_format("data:text/html,<script>"));
    }

    #[test]
    fn test_origin_validator() {
        let validator = OriginValidator::new()
            .allow_origin("https://example.com")
            .allow_localhost(true);

        assert!(validator.is_allowed("https://example.com"));
        assert!(validator.is_allowed(""));
        assert!(!validator.is_allowed("https://evil.com"));
    }

    #[test]
    fn test_pattern_matching() {
        let validator = OriginValidator::new().allow_pattern("*.example.com");

        assert!(validator.is_allowed("https://api.example.com"));
        assert!(validator.is_allowed("https://www.example.com"));
        assert!(!validator.is_allowed("https://example.org"));
    }

    #[test]
    fn test_localhost_detection() {
        assert!(is_localhost_origin("http://localhost"));
        assert!(is_localhost_origin(""));
        assert!(is_localhost_origin("https://localhost:8443"));
        assert!(is_localhost_origin("http://127.0.0.1"));
        assert!(is_localhost_origin("http://127.0.0.1:9000"));
        assert!(!is_localhost_origin("http://example.com"));
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(extract_host("https://example.com"), Some("example.com"));
        assert_eq!(extract_host("https://example.com:8443"), Some("example.com"));
        assert_eq!(extract_host(""), Some("localhost"));
        assert_eq!(extract_host("invalid"), None);
    }

    #[test]
    fn test_build_cors_layer() {
        let config = CorsConfig::development();
        let _layer = config.build();
    }

    #[test]
    fn test_dangerous_patterns_blocked() {
        assert!(!is_valid_origin_format("https://example.com<script>"));
        assert!(!is_valid_origin_format("javascript:void(0)"));
        assert!(!is_valid_origin_format("https://example.com%3cscript%3e"));
    }
}

mod part1;
mod part2;
mod part3;

use part1::*;
pub use part2::*;
