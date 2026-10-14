use super::*;

impl KRStatus {
    pub fn from_label(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "at_risk" => Self::AtRisk,
            "completed" => Self::Completed,
            _ => Self::NotStarted,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::AtRisk => "at_risk",
            Self::Completed => "completed",
        }
    }
}
