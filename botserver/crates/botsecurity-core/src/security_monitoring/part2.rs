use super::*;

impl SecurityEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LoginAttempt => "login_attempt",
            Self::LoginSuccess => "login_success",
            Self::LoginFailure => "login_failure",
            Self::PasswordReset => "password_reset",
            Self::MfaChallenge => "mfa_challenge",
            Self::MfaFailure => "mfa_failure",
            Self::SessionCreated => "session_created",
            Self::SessionRevoked => "session_revoked",
            Self::PermissionDenied => "permission_denied",
            Self::RateLimitExceeded => "rate_limit_exceeded",
            Self::SuspiciousActivity => "suspicious_activity",
            Self::BruteForceDetected => "brute_force_detected",
            Self::AccountLocked => "account_locked",
            Self::IpBlocked => "ip_blocked",
            Self::GeoAnomalyDetected => "geo_anomaly_detected",
            Self::ImpossibleTravel => "impossible_travel",
            Self::NewDeviceLogin => "new_device_login",
            Self::ApiKeyUsed => "api_key_used",
            Self::PrivilegeEscalation => "privilege_escalation",
            Self::DataExfiltration => "data_exfiltration",
        }
    }

    pub fn severity(&self) -> AlertSeverity {
        match self {
            Self::BruteForceDetected
            | Self::AccountLocked
            | Self::PrivilegeEscalation
            | Self::DataExfiltration => AlertSeverity::Critical,
            Self::LoginFailure
            | Self::MfaFailure
            | Self::PermissionDenied
            | Self::ImpossibleTravel
            | Self::GeoAnomalyDetected => AlertSeverity::High,
            Self::RateLimitExceeded
            | Self::SuspiciousActivity
            | Self::IpBlocked
            | Self::NewDeviceLogin => AlertSeverity::Medium,
            _ => AlertSeverity::Low,
        }
    }
}
