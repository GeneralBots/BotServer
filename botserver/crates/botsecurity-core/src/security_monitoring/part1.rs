use super::*;

impl Default for SecurityMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            brute_force_threshold: DEFAULT_BRUTE_FORCE_THRESHOLD,
            brute_force_window_seconds: DEFAULT_BRUTE_FORCE_WINDOW_SECONDS,
            lockout_duration_minutes: DEFAULT_LOCKOUT_DURATION_MINUTES,
            anomaly_detection_enabled: true,
            anomaly_threshold_stddev: DEFAULT_ANOMALY_THRESHOLD,
            geo_anomaly_detection: true,
            impossible_travel_detection: true,
            max_travel_speed_kmh: 1000.0,
            alert_on_critical: true,
            alert_on_high: true,
            retention_hours: 168,
        }
    }
}
