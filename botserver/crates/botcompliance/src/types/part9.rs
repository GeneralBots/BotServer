use super::*;

impl std::fmt::Display for TrainingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::SecurityAwareness => "security_awareness",
            Self::DataProtection => "data_protection",
            Self::IncidentResponse => "incident_response",
            Self::ComplianceOverview => "compliance_overview",
            Self::RoleSpecific => "role_specific",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for TrainingType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "security_awareness" => Ok(Self::SecurityAwareness),
            "data_protection" => Ok(Self::DataProtection),
            "incident_response" => Ok(Self::IncidentResponse),
            "compliance_overview" => Ok(Self::ComplianceOverview),
            "role_specific" => Ok(Self::RoleSpecific),
            _ => Err(format!("Unknown training type: {s}")),
        }
    }
}
