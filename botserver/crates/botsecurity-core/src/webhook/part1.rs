use super::*;

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timestamp_tolerance_seconds: DEFAULT_TIMESTAMP_TOLERANCE_SECONDS,
            replay_window_seconds: DEFAULT_REPLAY_WINDOW_SECONDS,
            require_https: true,
            max_payload_size: 1024 * 1024,
            retry_count: 3,
            retry_delay_seconds: 60,
            timeout_seconds: 30,
        }
    }
}
