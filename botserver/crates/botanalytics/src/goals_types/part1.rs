use super::*;

impl ObjectiveStatus {
    pub fn from_label(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "on_track" => Self::OnTrack,
            "at_risk" => Self::AtRisk,
            "behind" => Self::Behind,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            _ => Self::Draft,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::OnTrack => "on_track",
            Self::AtRisk => "at_risk",
            Self::Behind => "behind",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}
