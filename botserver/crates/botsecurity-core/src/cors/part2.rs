use super::*;

pub fn set_cors_allowed_origins(origins: Vec<String>) {
    if let Ok(mut guard) = CORS_ALLOWED_ORIGINS.write() {
        info!("Setting CORS allowed origins: {:?}", origins);
        *guard = Some(origins);
    }
}
pub fn get_cors_allowed_origins() -> Vec<String> {
    get_allowed_origins_from_config()
}
pub(super) fn validate_origin(origin: &HeaderValue, _request: &axum::http::request::Parts) -> bool {
    let origin_str = match origin.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    if origin_str.is_empty() {
        return false;
    }

    let config_origins = get_allowed_origins_from_config();
    if !config_origins.is_empty() {
        return config_origins.iter().any(|allowed| allowed == origin_str);
    }

    if is_valid_origin_format(origin_str) {
        return true;
    }

    false
}
pub(super) fn is_valid_origin_format(origin: &str) -> bool {
    // A request without an `Origin` header is same-origin by definition (a
    // direct navigation, or a non-browser client). There is no foreign origin
    // to validate, so the format check passes.
    if origin.is_empty() {
        return true;
    }

    if !origin.starts_with("http://") && !origin.starts_with("https://") {
        return false;
    }

    if origin.contains("..") || origin.matches("//").count() > 1 {
        return false;
    }

    let dangerous_patterns = [
        "<script",
        "javascript:",
        "data:",
        "vbscript:",
        "%3c",
        "%3e",
        "\\x",
        "\\u",
    ];

    let origin_lower = origin.to_lowercase();
    for pattern in &dangerous_patterns {
        if origin_lower.contains(pattern) {
            return false;
        }
    }

    true
}
pub fn create_cors_layer() -> CorsLayer {
    let config_origins = get_allowed_origins_from_config();

    if !config_origins.is_empty() {
        info!("Creating CORS layer with configured origins");
        CorsConfig::production().with_origins(config_origins).build()
    } else {
        info!("Creating CORS layer with permissive defaults (no origins configured — allowing any valid HTTP/S origin)");
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(validate_origin))
            .allow_methods([
                Method::GET, Method::POST, Method::PUT,
                Method::DELETE, Method::PATCH, Method::OPTIONS, Method::HEAD,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
                header::ORIGIN,
                header::HeaderName::from_static("x-request-id"),
                header::HeaderName::from_static("x-user-id"),
            ])
            .expose_headers([
                header::HeaderName::from_static("x-request-id"),
                header::HeaderName::from_static("x-ratelimit-limit"),
                header::HeaderName::from_static("x-ratelimit-remaining"),
                header::HeaderName::from_static("retry-after"),
            ])
            .allow_credentials(true)
            .max_age(std::time::Duration::from_secs(7200))
    }
}
/// Middleware that strips CORS headers from responses when the request
/// came through a reverse proxy (detected via X-Forwarded-* headers).
/// This prevents duplicate Access-Control-Allow-Origin headers caused by
/// both the application and the proxy (e.g., Caddy) setting them.
pub async fn strip_proxy_cors_middleware(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let behind_proxy = request.headers().contains_key("x-forwarded-for")
        || request.headers().contains_key("x-forwarded-host")
        || request.headers().contains_key("x-forwarded-proto");

    let mut response = next.run(request).await;

    if behind_proxy {
        let headers = response.headers_mut();
        headers.remove(header::ACCESS_CONTROL_ALLOW_ORIGIN);
        headers.remove(header::ACCESS_CONTROL_ALLOW_METHODS);
        headers.remove(header::ACCESS_CONTROL_ALLOW_HEADERS);
        headers.remove(header::ACCESS_CONTROL_EXPOSE_HEADERS);
        headers.remove(header::ACCESS_CONTROL_MAX_AGE);
        headers.remove(header::ACCESS_CONTROL_ALLOW_CREDENTIALS);
    }

    response
}
pub fn create_cors_layer_for_production(allowed_origins: Vec<String>) -> CorsLayer {
    if allowed_origins.is_empty() {
        CorsConfig::production().build()
    } else {
        CorsConfig::production().with_origins(allowed_origins).build()
    }
}
pub fn create_cors_layer_with_origins(origins: Vec<String>) -> CorsLayer {
    CorsConfig::production().with_origins(origins).build()
}
