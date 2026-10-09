use super::*;

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            idle_timeout_minutes: 30,
            absolute_timeout_hours: 24,
            max_concurrent_sessions: 5,
            session_id_length: 32,
            cookie_name: "gb_session".into(),
            cookie_secure: true,
            cookie_http_only: true,
            cookie_same_site: SameSite::Strict,
            enable_device_tracking: true,
            enable_ip_tracking: true,
        }
    }
}
