use super::*;

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            content_security_policy: Some(
                "default-src 'self'; \
                 script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
                 style-src 'self' 'unsafe-inline'; \
                 img-src 'self' data: https:; \
                 font-src 'self' data:; \
                 connect-src 'self' wss: https:; \
                 frame-ancestors 'self'; \
                 base-uri 'self'; \
                 form-action 'self'"
                    .to_string(),
            ),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
            strict_transport_security: Some("max-age=31536000; includeSubDomains; preload".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some(
                "accelerometer=(), \
                 camera=(), \
                 geolocation=(), \
                 gyroscope=(), \
                 magnetometer=(), \
                 microphone=(), \
                 payment=(), \
                 usb=()"
                    .to_string(),
            ),
            cache_control: Some("no-store, no-cache, must-revalidate, proxy-revalidate".to_string()),
            custom_headers: HashMap::new(),
        }
    }
}
impl SecurityHeadersConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn strict() -> Self {
        Self {
            content_security_policy: Some(
                "default-src 'self'; \
                 script-src 'self'; \
                 style-src 'self'; \
                 img-src 'self'; \
                 font-src 'self'; \
                 connect-src 'self'; \
                 frame-ancestors 'none'; \
                 base-uri 'self'; \
                 form-action 'self'; \
                 upgrade-insecure-requests"
                    .to_string(),
            ),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
            strict_transport_security: Some(
                "max-age=63072000; includeSubDomains; preload".to_string(),
            ),
            referrer_policy: Some("no-referrer".to_string()),
            permissions_policy: Some(
                "accelerometer=(), \
                 ambient-light-sensor=(), \
                 autoplay=(), \
                 battery=(), \
                 camera=(), \
                 cross-origin-isolated=(), \
                 display-capture=(), \
                 document-domain=(), \
                 encrypted-media=(), \
                 execution-while-not-rendered=(), \
                 execution-while-out-of-viewport=(), \
                 fullscreen=(), \
                 geolocation=(), \
                 gyroscope=(), \
                 keyboard-map=(), \
                 magnetometer=(), \
                 microphone=(), \
                 midi=(), \
                 navigation-override=(), \
                 payment=(), \
                 picture-in-picture=(), \
                 publickey-credentials-get=(), \
                 screen-wake-lock=(), \
                 sync-xhr=(), \
                 usb=(), \
                 web-share=(), \
                 xr-spatial-tracking=()"
                    .to_string(),
            ),
            cache_control: Some(
                "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0".to_string(),
            ),
            custom_headers: HashMap::from([
                ("X-Permitted-Cross-Domain-Policies".to_string(), "none".to_string()),
                ("Cross-Origin-Embedder-Policy".to_string(), "require-corp".to_string()),
                ("Cross-Origin-Opener-Policy".to_string(), "same-origin".to_string()),
                ("Cross-Origin-Resource-Policy".to_string(), "same-origin".to_string()),
            ]),
        }
    }

    pub fn relaxed() -> Self {
        Self {
            content_security_policy: None,
            x_frame_options: Some("SAMEORIGIN".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
            strict_transport_security: Some("max-age=31536000".to_string()),
            referrer_policy: Some("origin-when-cross-origin".to_string()),
            permissions_policy: None,
            cache_control: None,
            custom_headers: HashMap::new(),
        }
    }

    pub fn api() -> Self {
        Self {
            content_security_policy: Some("default-src 'none'; frame-ancestors 'none'".to_string()),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("0".to_string()),
            strict_transport_security: Some("max-age=31536000; includeSubDomains".to_string()),
            referrer_policy: Some("no-referrer".to_string()),
            permissions_policy: None,
            cache_control: Some("no-store".to_string()),
            custom_headers: HashMap::from([
                ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
            ]),
        }
    }

    pub fn with_csp(mut self, policy: impl Into<String>) -> Self {
        self.content_security_policy = Some(policy.into());
        self
    }

    pub fn without_csp(mut self) -> Self {
        self.content_security_policy = None;
        self
    }

    pub fn with_frame_options(mut self, options: impl Into<String>) -> Self {
        self.x_frame_options = Some(options.into());
        self
    }

    pub fn with_hsts(mut self, max_age: u64, include_subdomains: bool, preload: bool) -> Self {
        let mut value = format!("max-age={}", max_age);
        if include_subdomains {
            value.push_str("; includeSubDomains");
        }
        if preload {
            value.push_str("; preload");
        }
        self.strict_transport_security = Some(value);
        self
    }

    pub fn with_referrer_policy(mut self, policy: impl Into<String>) -> Self {
        self.referrer_policy = Some(policy.into());
        self
    }

    pub fn with_custom_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_headers.insert(name.into(), value.into());
        self
    }

    pub fn disable_hsts(mut self) -> Self {
        self.strict_transport_security = None;
        self
    }
}
pub async fn security_headers_middleware(
    axum::Extension(config): axum::Extension<SecurityHeadersConfig>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    apply_security_headers(&mut response, &config);
    response
}
pub async fn security_headers_middleware_default(
    request: Request<Body>,
    next: Next,
) -> Response {
    let config = SecurityHeadersConfig::default();
    let mut response = next.run(request).await;
    apply_security_headers(&mut response, &config);
    response
}
pub(super) fn apply_security_headers(response: &mut Response, config: &SecurityHeadersConfig) {
    let headers = response.headers_mut();

    if let Some(ref csp) = config.content_security_policy {
        if let Ok(value) = HeaderValue::from_str(csp) {
            headers.insert(
                HeaderName::from_static("content-security-policy"),
                value,
            );
        }
    }

    if let Some(ref xfo) = config.x_frame_options {
        if let Ok(value) = HeaderValue::from_str(xfo) {
            headers.insert(
                HeaderName::from_static("x-frame-options"),
                value,
            );
        }
    }

    if let Some(ref xcto) = config.x_content_type_options {
        if let Ok(value) = HeaderValue::from_str(xcto) {
            headers.insert(
                HeaderName::from_static("x-content-type-options"),
                value,
            );
        }
    }

    if let Some(ref xxp) = config.x_xss_protection {
        if let Ok(value) = HeaderValue::from_str(xxp) {
            headers.insert(
                HeaderName::from_static("x-xss-protection"),
                value,
            );
        }
    }

    if let Some(ref hsts) = config.strict_transport_security {
        if let Ok(value) = HeaderValue::from_str(hsts) {
            headers.insert(
                HeaderName::from_static("strict-transport-security"),
                value,
            );
        }
    }

    if let Some(ref rp) = config.referrer_policy {
        if let Ok(value) = HeaderValue::from_str(rp) {
            headers.insert(
                HeaderName::from_static("referrer-policy"),
                value,
            );
        }
    }

    if let Some(ref pp) = config.permissions_policy {
        if let Ok(value) = HeaderValue::from_str(pp) {
            headers.insert(
                HeaderName::from_static("permissions-policy"),
                value,
            );
        }
    }

    if let Some(ref cc) = config.cache_control {
        if let Ok(value) = HeaderValue::from_str(cc) {
            headers.insert(
                HeaderName::from_static("cache-control"),
                value,
            );
        }
    }

    for (name, value) in &config.custom_headers {
        if let (Ok(header_name), Ok(header_value)) = (
            HeaderName::try_from(name.to_lowercase()),
            HeaderValue::from_str(value),
        ) {
            headers.insert(header_name, header_value);
        }
    }

    headers.insert(
        HeaderName::from_static("x-powered-by"),
        HeaderValue::from_static("General Bots"),
    );
}
pub fn create_security_headers_layer(
    config: SecurityHeadersConfig,
) -> axum::Extension<SecurityHeadersConfig> {
    axum::Extension(config)
}
