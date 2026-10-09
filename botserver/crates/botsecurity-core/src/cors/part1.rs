use super::*;

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Request-ID".to_string(),
                "X-User-ID".to_string(),
                "Accept".to_string(),
                "Accept-Language".to_string(),
                "Origin".to_string(),
            ],
            exposed_headers: vec![
                "X-Request-ID".to_string(),
                "X-RateLimit-Limit".to_string(),
                "X-RateLimit-Remaining".to_string(),
                "X-RateLimit-Reset".to_string(),
                "Retry-After".to_string(),
            ],
            allow_credentials: true,
            max_age_secs: 3600,
        }
    }
}
impl CorsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_config_value(allowed_origins: Option<&str>) -> Self {
        let mut config = Self::production();

        if let Some(origins) = allowed_origins {
            let origins: Vec<String> = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if !origins.is_empty() {
                info!("CORS configured with {} allowed origins", origins.len());
                config.allowed_origins = origins;
            }
        }

        config
    }

    pub fn production() -> Self {
        Self {
            allowed_origins: vec![],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Request-ID".to_string(),
            ],
            exposed_headers: vec![
                "X-Request-ID".to_string(),
                "X-RateLimit-Limit".to_string(),
                "X-RateLimit-Remaining".to_string(),
                "Retry-After".to_string(),
            ],
            allow_credentials: true,
            max_age_secs: 7200,
        }
    }

    pub fn development() -> Self {
        Self {
            allowed_origins: vec![
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "http://127.0.0.1:8080".to_string(),
                "http://127.0.0.1:9000".to_string(),
                "https://localhost:3000".to_string(),
                "https://localhost:8080".to_string(),
                "https://localhost:9000".to_string(),
            ],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
                Method::HEAD,
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Request-ID".to_string(),
                "X-User-ID".to_string(),
                "Accept".to_string(),
                "Accept-Language".to_string(),
                "Origin".to_string(),
                "X-Debug".to_string(),
            ],
            exposed_headers: vec![
                "X-Request-ID".to_string(),
                "X-RateLimit-Limit".to_string(),
                "X-RateLimit-Remaining".to_string(),
                "X-RateLimit-Reset".to_string(),
                "Retry-After".to_string(),
                "X-Debug-Info".to_string(),
            ],
            allow_credentials: true,
            max_age_secs: 3600,
        }
    }

    pub fn api() -> Self {
        Self {
            allowed_origins: vec![],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Request-ID".to_string(),
                "X-API-Key".to_string(),
            ],
            exposed_headers: vec![
                "X-Request-ID".to_string(),
                "X-RateLimit-Limit".to_string(),
                "X-RateLimit-Remaining".to_string(),
                "Retry-After".to_string(),
            ],
            allow_credentials: false,
            max_age_secs: 86400,
        }
    }

    pub fn with_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    pub fn add_origin(mut self, origin: impl Into<String>) -> Self {
        self.allowed_origins.push(origin.into());
        self
    }

    pub fn with_methods(mut self, methods: Vec<Method>) -> Self {
        self.allowed_methods = methods;
        self
    }

    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.allowed_headers = headers;
        self
    }

    pub fn add_header(mut self, header: impl Into<String>) -> Self {
        self.allowed_headers.push(header.into());
        self
    }

    pub fn with_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    pub fn with_max_age(mut self, secs: u64) -> Self {
        self.max_age_secs = secs;
        self
    }

    pub fn build(self) -> CorsLayer {
        let mut cors = CorsLayer::new();

        if self.allowed_origins.is_empty() {
            let allowed_env_origins = get_allowed_origins_from_config();
            if allowed_env_origins.is_empty() {
                cors = cors.allow_origin(AllowOrigin::predicate(validate_origin));
            } else {
                let origins: Vec<HeaderValue> = allowed_env_origins
                    .iter()
                    .filter_map(|o| o.parse().ok())
                    .collect();
                if origins.is_empty() {
                    cors = cors.allow_origin(AllowOrigin::predicate(validate_origin));
                } else {
                    cors = cors.allow_origin(origins);
                }
            }
        } else {
            let origins: Vec<HeaderValue> = self
                .allowed_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();
            if origins.is_empty() {
                cors = cors.allow_origin(AllowOrigin::predicate(validate_origin));
            } else {
                cors = cors.allow_origin(origins);
            }
        }

        cors = cors.allow_methods(self.allowed_methods);

        let headers: Vec<header::HeaderName> = self
            .allowed_headers
            .iter()
            .filter_map(|h| h.parse().ok())
            .collect();
        cors = cors.allow_headers(headers);

        let exposed: Vec<header::HeaderName> = self
            .exposed_headers
            .iter()
            .filter_map(|h| h.parse().ok())
            .collect();
        cors = cors.expose_headers(exposed);

        if self.allow_credentials {
            cors = cors.allow_credentials(true);
        }

        cors = cors.max_age(std::time::Duration::from_secs(self.max_age_secs));

        cors
    }
}
pub(super) fn get_allowed_origins_from_config() -> Vec<String> {
    if let Some(origins) = CORS_ALLOWED_ORIGINS.read().ok().and_then(|g| g.clone()) {
        return origins;
    }
    Vec::new()
}
