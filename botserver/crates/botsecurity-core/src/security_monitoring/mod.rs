pub(crate) use chrono::{DateTime, Duration, Timelike, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::{info, warn};
pub(crate) use uuid::Uuid;

const DEFAULT_BRUTE_FORCE_THRESHOLD: u32 = 5;
const DEFAULT_BRUTE_FORCE_WINDOW_SECONDS: i64 = 300;
const DEFAULT_LOCKOUT_DURATION_MINUTES: i64 = 30;
const DEFAULT_ANOMALY_THRESHOLD: f64 = 3.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMonitoringConfig {
    pub enabled: bool,
    pub brute_force_threshold: u32,
    pub brute_force_window_seconds: i64,
    pub lockout_duration_minutes: i64,
    pub anomaly_detection_enabled: bool,
    pub anomaly_threshold_stddev: f64,
    pub geo_anomaly_detection: bool,
    pub impossible_travel_detection: bool,
    pub max_travel_speed_kmh: f64,
    pub alert_on_critical: bool,
    pub alert_on_high: bool,
    pub retention_hours: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityEventType {
    LoginAttempt,
    LoginSuccess,
    LoginFailure,
    PasswordReset,
    MfaChallenge,
    MfaFailure,
    SessionCreated,
    SessionRevoked,
    PermissionDenied,
    RateLimitExceeded,
    SuspiciousActivity,
    BruteForceDetected,
    AccountLocked,
    IpBlocked,
    GeoAnomalyDetected,
    ImpossibleTravel,
    NewDeviceLogin,
    ApiKeyUsed,
    PrivilegeEscalation,
    DataExfiltration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEventType,
    pub severity: AlertSeverity,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub location: Option<GeoLocation>,
    pub device_fingerprint: Option<String>,
    pub details: HashMap<String, serde_json::Value>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginAttemptRecord {
    pub user_id: Option<Uuid>,
    pub ip_address: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub user_agent: Option<String>,
    pub location: Option<GeoLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutRecord {
    pub identifier: String,
    pub locked_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub reason: String,
    pub attempt_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlert {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub event_ids: Vec<Uuid>,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub acknowledged: bool,
    pub acknowledged_by: Option<Uuid>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSecurityProfile {
    pub user_id: Uuid,
    pub known_ips: Vec<String>,
    pub known_devices: Vec<String>,
    pub known_locations: Vec<GeoLocation>,
    pub last_login: Option<DateTime<Utc>>,
    pub last_location: Option<GeoLocation>,
    pub login_times: Vec<u32>,
    pub risk_score: f64,
    pub is_locked: bool,
    pub lock_expires_at: Option<DateTime<Utc>>,
}

pub struct SecurityMonitor {
    config: SecurityMonitoringConfig,
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    login_attempts: Arc<RwLock<HashMap<String, Vec<LoginAttemptRecord>>>>,
    lockouts: Arc<RwLock<HashMap<String, LockoutRecord>>>,
    alerts: Arc<RwLock<Vec<SecurityAlert>>>,
    user_profiles: Arc<RwLock<HashMap<Uuid, UserSecurityProfile>>>,
    blocked_ips: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_login_success() {
        let monitor = SecurityMonitor::with_defaults();
        let user_id = Uuid::new_v4();

        let alert = monitor
            .record_login_attempt(Some(user_id), "192.168.1.1", true, Some("TestAgent"), None)
            .await;

        assert!(alert.is_none());

        let events = monitor.get_recent_events(Some(SecurityEventType::LoginSuccess), None, 10).await;
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_brute_force_detection() {
        let config = SecurityMonitoringConfig {
            brute_force_threshold: 3,
            ..Default::default()
        };
        let monitor = SecurityMonitor::new(config);
        let user_id = Uuid::new_v4();

        for _ in 0..2 {
            let alert = monitor
                .record_login_attempt(Some(user_id), "10.0.0.1", false, None, None)
                .await;
            assert!(alert.is_none());
        }

        let alert = monitor
            .record_login_attempt(Some(user_id), "10.0.0.1", false, None, None)
            .await;

        assert!(alert.is_some());
        assert_eq!(alert.unwrap().severity, AlertSeverity::Critical);
    }

    #[tokio::test]
    async fn test_lockout() {
        let config = SecurityMonitoringConfig {
            brute_force_threshold: 2,
            ..Default::default()
        };
        let monitor = SecurityMonitor::new(config);

        let user_id = Uuid::new_v4();
        let identifier = user_id.to_string();

        monitor
            .record_login_attempt(Some(user_id), "10.0.0.1", false, None, None)
            .await;
        monitor
            .record_login_attempt(Some(user_id), "10.0.0.1", false, None, None)
            .await;

        assert!(monitor.is_locked(&identifier).await);

        monitor.unlock(&identifier).await;
        assert!(!monitor.is_locked(&identifier).await);
    }

    #[tokio::test]
    async fn test_ip_blocking() {
        let monitor = SecurityMonitor::with_defaults();

        monitor
            .block_ip("1.2.3.4", Duration::minutes(30), "Test block")
            .await;

        assert!(monitor.is_ip_blocked("1.2.3.4").await);
        assert!(!monitor.is_ip_blocked("5.6.7.8").await);

        monitor.unblock_ip("1.2.3.4").await;
        assert!(!monitor.is_ip_blocked("1.2.3.4").await);
    }

    #[test]
    fn test_security_event_creation() {
        let event = SecurityEvent::new(SecurityEventType::LoginFailure)
            .with_user(Uuid::new_v4())
            .with_ip("192.168.1.1".into())
            .with_detail("reason", serde_json::json!("invalid_password"));

        assert_eq!(event.event_type, SecurityEventType::LoginFailure);
        assert!(event.user_id.is_some());
        assert!(event.ip_address.is_some());
    }

    #[test]
    fn test_alert_creation() {
        let mut alert = SecurityAlert::new(
            AlertSeverity::High,
            "Test Alert",
            "Test description",
        );

        assert!(!alert.acknowledged);
        assert!(!alert.resolved);

        alert.acknowledge(Uuid::new_v4());
        assert!(alert.acknowledged);

        alert.resolve();
        assert!(alert.resolved);
    }

    #[test]
    fn test_geo_location_distance() {
        let loc1 = GeoLocation::new()
            .with_coordinates(40.7128, -74.0060);
        let loc2 = GeoLocation::new()
            .with_coordinates(51.5074, -0.1278);

        let distance = loc1.distance_km(&loc2).unwrap();
        assert!(distance > 5500.0 && distance < 5600.0);
    }

    #[test]
    fn test_haversine_distance() {
        let distance = haversine_distance(40.7128, -74.0060, 51.5074, -0.1278);
        assert!(distance > 5500.0 && distance < 5600.0);
    }

    #[test]
    fn test_user_security_profile() {
        let user_id = Uuid::new_v4();
        let mut profile = UserSecurityProfile::new(user_id);

        assert!(!profile.is_known_ip("192.168.1.1"));

        profile.add_known_ip("192.168.1.1");
        assert!(profile.is_known_ip("192.168.1.1"));

        profile.lock(Duration::minutes(30));
        assert!(profile.check_lock_status());

        profile.unlock();
        assert!(!profile.check_lock_status());
    }

    #[test]
    fn test_lockout_record() {
        let lockout = LockoutRecord {
            identifier: "test".into(),
            locked_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(30),
            reason: "Test".into(),
            attempt_count: 5,
        };

        assert!(!lockout.is_expired());
        assert!(lockout.remaining_time() > Duration::zero());
    }

    #[test]
    fn test_event_severity_mapping() {
        assert_eq!(
            SecurityEventType::BruteForceDetected.severity(),
            AlertSeverity::Critical
        );
        assert_eq!(
            SecurityEventType::LoginFailure.severity(),
            AlertSeverity::High
        );
        assert_eq!(
            SecurityEventType::LoginSuccess.severity(),
            AlertSeverity::Low
        );
    }

    #[tokio::test]
    async fn test_alert_acknowledgment() {
        let monitor = SecurityMonitor::with_defaults();

        let event = SecurityEvent::new(SecurityEventType::BruteForceDetected);
        monitor.record_event(event).await;

        let alerts = monitor.get_alerts(true, 10).await;
        assert!(!alerts.is_empty());

        let alert_id = alerts[0].id;
        let admin_id = Uuid::new_v4();

        assert!(monitor.acknowledge_alert(alert_id, admin_id).await);

        let unack_alerts = monitor.get_alerts(true, 10).await;
        assert!(unack_alerts.iter().all(|a| a.id != alert_id));
    }

    #[tokio::test]
    async fn test_cleanup() {
        let config = SecurityMonitoringConfig {
            retention_hours: 0,
            ..Default::default()
        };
        let monitor = SecurityMonitor::new(config);

        let event = SecurityEvent::new(SecurityEventType::LoginSuccess);
        monitor.record_event(event).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let cleaned = monitor.cleanup_old_data().await;
        assert!(cleaned >= 1);
    }

    #[test]
    fn test_alert_severity_score() {
        assert_eq!(AlertSeverity::Low.score(), 25);
        assert_eq!(AlertSeverity::Medium.score(), 50);
        assert_eq!(AlertSeverity::High.score(), 75);
        assert_eq!(AlertSeverity::Critical.score(), 100);
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;
mod part11;

use part11::*;
