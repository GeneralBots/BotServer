use super::*;

impl AuditEventType {
    pub fn category(&self) -> AuditEventCategory {
        match self {
            Self::LoginSuccess
            | Self::LoginFailure
            | Self::Logout
            | Self::MfaEnabled
            | Self::MfaDisabled
            | Self::MfaChallenge
            | Self::PasswordChange
            | Self::PasswordReset
            | Self::SessionCreate
            | Self::SessionRevoke
            | Self::SessionExpire => AuditEventCategory::Authentication,

            Self::PermissionGranted
            | Self::PermissionDenied
            | Self::RoleAssigned
            | Self::RoleRemoved
            | Self::GroupJoined
            | Self::GroupLeft => AuditEventCategory::Authorization,

            Self::DataRead | Self::DataDownload => AuditEventCategory::DataAccess,

            Self::DataCreate
            | Self::DataUpdate
            | Self::DataDelete
            | Self::DataShare
            | Self::DataUnshare
            | Self::DataExport => AuditEventCategory::DataModification,

            Self::UserCreate
            | Self::UserDelete
            | Self::UserModify
            | Self::UserDisable
            | Self::UserEnable
            | Self::ConfigChange
            | Self::SettingChange
            | Self::BotCreate
            | Self::BotDelete
            | Self::BotModify => AuditEventCategory::Administration,

            Self::ThreatDetected
            | Self::PolicyViolation
            | Self::RateLimitExceeded
            | Self::SuspiciousActivity
            | Self::BruteForceAttempt
            | Self::InjectionAttempt
            | Self::UnauthorizedAccess => AuditEventCategory::Security,

            Self::ServiceStart
            | Self::ServiceStop
            | Self::BackupCreate
            | Self::BackupRestore
            | Self::MaintenanceStart
            | Self::MaintenanceEnd => AuditEventCategory::System,

            Self::ConsentGiven
            | Self::ConsentWithdrawn
            | Self::DataExportRequest
            | Self::DataDeletionRequest
            | Self::PrivacyPolicyAccepted => AuditEventCategory::Compliance,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LoginSuccess => "LOGIN_SUCCESS",
            Self::LoginFailure => "LOGIN_FAILURE",
            Self::Logout => "LOGOUT",
            Self::MfaEnabled => "MFA_ENABLED",
            Self::MfaDisabled => "MFA_DISABLED",
            Self::MfaChallenge => "MFA_CHALLENGE",
            Self::PasswordChange => "PASSWORD_CHANGE",
            Self::PasswordReset => "PASSWORD_RESET",
            Self::SessionCreate => "SESSION_CREATE",
            Self::SessionRevoke => "SESSION_REVOKE",
            Self::SessionExpire => "SESSION_EXPIRE",
            Self::PermissionGranted => "PERMISSION_GRANTED",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::RoleAssigned => "ROLE_ASSIGNED",
            Self::RoleRemoved => "ROLE_REMOVED",
            Self::GroupJoined => "GROUP_JOINED",
            Self::GroupLeft => "GROUP_LEFT",
            Self::DataRead => "DATA_READ",
            Self::DataCreate => "DATA_CREATE",
            Self::DataUpdate => "DATA_UPDATE",
            Self::DataDelete => "DATA_DELETE",
            Self::DataShare => "DATA_SHARE",
            Self::DataUnshare => "DATA_UNSHARE",
            Self::DataDownload => "DATA_DOWNLOAD",
            Self::DataExport => "DATA_EXPORT",
            Self::UserCreate => "USER_CREATE",
            Self::UserDelete => "USER_DELETE",
            Self::UserModify => "USER_MODIFY",
            Self::UserDisable => "USER_DISABLE",
            Self::UserEnable => "USER_ENABLE",
            Self::ConfigChange => "CONFIG_CHANGE",
            Self::SettingChange => "SETTING_CHANGE",
            Self::BotCreate => "BOT_CREATE",
            Self::BotDelete => "BOT_DELETE",
            Self::BotModify => "BOT_MODIFY",
            Self::ThreatDetected => "THREAT_DETECTED",
            Self::PolicyViolation => "POLICY_VIOLATION",
            Self::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            Self::SuspiciousActivity => "SUSPICIOUS_ACTIVITY",
            Self::BruteForceAttempt => "BRUTE_FORCE_ATTEMPT",
            Self::InjectionAttempt => "INJECTION_ATTEMPT",
            Self::UnauthorizedAccess => "UNAUTHORIZED_ACCESS",
            Self::ServiceStart => "SERVICE_START",
            Self::ServiceStop => "SERVICE_STOP",
            Self::BackupCreate => "BACKUP_CREATE",
            Self::BackupRestore => "BACKUP_RESTORE",
            Self::MaintenanceStart => "MAINTENANCE_START",
            Self::MaintenanceEnd => "MAINTENANCE_END",
            Self::ConsentGiven => "CONSENT_GIVEN",
            Self::ConsentWithdrawn => "CONSENT_WITHDRAWN",
            Self::DataExportRequest => "DATA_EXPORT_REQUEST",
            Self::DataDeletionRequest => "DATA_DELETION_REQUEST",
            Self::PrivacyPolicyAccepted => "PRIVACY_POLICY_ACCEPTED",
        }
    }

    pub fn severity(&self) -> AuditSeverity {
        match self {
            Self::LoginFailure
            | Self::PermissionDenied
            | Self::UnauthorizedAccess
            | Self::RateLimitExceeded => AuditSeverity::Warning,

            Self::ThreatDetected
            | Self::PolicyViolation
            | Self::BruteForceAttempt
            | Self::InjectionAttempt
            | Self::SuspiciousActivity => AuditSeverity::Critical,

            Self::UserDelete
            | Self::DataDelete
            | Self::PasswordChange
            | Self::PasswordReset
            | Self::MfaDisabled
            | Self::ConfigChange
            | Self::BotDelete => AuditSeverity::High,

            _ => AuditSeverity::Info,
        }
    }
}
