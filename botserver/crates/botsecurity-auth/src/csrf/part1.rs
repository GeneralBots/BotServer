use super::*;

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            token_expiry_minutes: DEFAULT_TOKEN_EXPIRY_MINUTES,
            cookie_name: "csrf_token".into(),
            header_name: "X-CSRF-Token".into(),
            form_field_name: "_csrf".into(),
            cookie_secure: true,
            cookie_same_site: SameSite::Strict,
            exempt_paths: vec!["/api/health".into(), "/api/version".into()],
            exempt_methods: vec!["GET".into(), "HEAD".into(), "OPTIONS".into()],
            double_submit_cookie: true,
        }
    }
}
