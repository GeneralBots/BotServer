pub(crate) use axum::{ body::Body, http::{header::HeaderName, HeaderValue, Request}, middleware::Next, response::Response, };
pub(crate) use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    pub content_security_policy: Option<String>,
    pub x_frame_options: Option<String>,
    pub x_content_type_options: Option<String>,
    pub x_xss_protection: Option<String>,
    pub strict_transport_security: Option<String>,
    pub referrer_policy: Option<String>,
    pub permissions_policy: Option<String>,
    pub cache_control: Option<String>,
    pub custom_headers: HashMap<String, String>,
}

pub struct CspBuilder {
    directives: HashMap<String, Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}, response::IntoResponse, routing::get, Router};
    use tower::ServiceExt;

    async fn dummy_handler() -> impl IntoResponse {
        (StatusCode::OK, "Hello, world!")
    }

    #[tokio::test]
    async fn test_security_headers_middleware_application() {
        let config = SecurityHeadersConfig::default();
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(axum::middleware::from_fn(security_headers_middleware))
            .layer(axum::Extension(config));
            
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        
        // Ensure standard security headers are applied
        assert!(response.headers().contains_key("content-security-policy"));
        assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
        assert_eq!(response.headers().get("x-content-type-options").unwrap(), "nosniff");
        assert_eq!(response.headers().get("x-powered-by").unwrap(), "General Bots");
    }

    #[test]
    fn test_default_config() {
        let config = SecurityHeadersConfig::default();
        assert!(config.content_security_policy.is_some());
        assert_eq!(config.x_frame_options, Some("DENY".to_string()));
        assert_eq!(config.x_content_type_options, Some("nosniff".to_string()));
    }

    #[test]
    fn test_strict_config() {
        let config = SecurityHeadersConfig::strict();
        assert!(config.content_security_policy.is_some());
        assert!(config.referrer_policy == Some("no-referrer".to_string()));
        assert!(!config.custom_headers.is_empty());
    }

    #[test]
    fn test_relaxed_config() {
        let config = SecurityHeadersConfig::relaxed();
        assert!(config.content_security_policy.is_none());
        assert_eq!(config.x_frame_options, Some("SAMEORIGIN".to_string()));
    }

    #[test]
    fn test_api_config() {
        let config = SecurityHeadersConfig::api();
        assert!(config.permissions_policy.is_none());
        assert_eq!(config.cache_control, Some("no-store".to_string()));
    }

    #[test]
    fn test_builder_methods() {
        let config = SecurityHeadersConfig::default()
            .with_csp("default-src 'self'")
            .with_frame_options("SAMEORIGIN")
            .with_hsts(63072000, true, true)
            .with_referrer_policy("no-referrer")
            .with_custom_header("X-Custom", "value");

        assert_eq!(
            config.content_security_policy,
            Some("default-src 'self'".to_string())
        );
        assert_eq!(config.x_frame_options, Some("SAMEORIGIN".to_string()));
        assert!(config
            .strict_transport_security
            .as_ref()
            .unwrap()
            .contains("63072000"));
        assert_eq!(config.referrer_policy, Some("no-referrer".to_string()));
        assert_eq!(
            config.custom_headers.get("X-Custom"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_csp_builder() {
        let csp = CspBuilder::new()
            .default_src(&["'self'"])
            .script_src(&["'self'", "'unsafe-inline'"])
            .style_src(&["'self'", "https://fonts.googleapis.com"])
            .img_src(&["'self'", "data:", "https:"])
            .upgrade_insecure_requests()
            .build();

        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("script-src 'self' 'unsafe-inline'"));
        assert!(csp.contains("upgrade-insecure-requests"));
    }

    #[test]
    fn test_csp_builder_empty_directive() {
        let csp = CspBuilder::new()
            .default_src(&["'none'"])
            .block_all_mixed_content()
            .build();

        assert!(csp.contains("block-all-mixed-content"));
        assert!(csp.contains("default-src 'none'"));
    }

    #[test]
    fn test_disable_hsts() {
        let config = SecurityHeadersConfig::default().disable_hsts();
        assert!(config.strict_transport_security.is_none());
    }

    #[test]
    fn test_without_csp() {
        let config = SecurityHeadersConfig::default().without_csp();
        assert!(config.content_security_policy.is_none());
    }
}

mod part1;
mod part2;

pub use part1::*;
