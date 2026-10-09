use super::*;

impl IncidentSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Informational => "informational",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "critical" => Some(Self::Critical),
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            "informational" | "info" => Some(Self::Informational),
            _ => None,
        }
    }

    pub fn response_time_minutes(&self) -> i64 {
        match self {
            Self::Critical => 15,
            Self::High => 60,
            Self::Medium => 240,
            Self::Low => 1440,
            Self::Informational => 10080,
        }
    }
}
