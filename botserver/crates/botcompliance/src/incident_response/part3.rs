use super::*;

impl IncidentCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SecurityBreach => "security_breach",
            Self::DataLeak => "data_leak",
            Self::ServiceOutage => "service_outage",
            Self::PerformanceDegradation => "performance_degradation",
            Self::UnauthorizedAccess => "unauthorized_access",
            Self::Malware => "malware",
            Self::PhishingAttack => "phishing_attack",
            Self::DenialOfService => "denial_of_service",
            Self::ConfigurationError => "configuration_error",
            Self::ComplianceViolation => "compliance_violation",
            Self::Other => "other",
        }
    }
}
