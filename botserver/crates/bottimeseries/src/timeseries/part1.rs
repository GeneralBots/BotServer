use super::*;

impl Default for TimeSeriesConfig {
    fn default() -> Self {
        Self {
            url: "".to_string(),
            token: String::new(),
            org: "system".to_string(),
            bucket: "metrics".to_string(),
            batch_size: 1000,
            flush_interval_ms: 1000,
            verify_tls: true,
        }
    }
}
