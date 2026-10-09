use super::*;

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            min_length: 12,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: true,
            min_unique_chars: 6,
            max_consecutive_chars: 3,
            password_history_count: 12,
            expiration_days: Some(90),
            lockout_threshold: 5,
            lockout_duration_minutes: 30,
        }
    }
}
