use super::*;

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            default_rate_limit_per_minute: 60,
            default_rate_limit_per_hour: 1000,
            default_rate_limit_per_day: 10000,
            max_keys_per_user: 10,
            key_expiry_days: Some(365),
            allow_key_rotation: true,
            rotation_grace_period_hours: 24,
        }
    }
}
