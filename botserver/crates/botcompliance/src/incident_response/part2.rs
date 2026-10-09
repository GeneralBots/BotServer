use super::*;

impl IncidentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Triaging => "triaging",
            Self::Investigating => "investigating",
            Self::Containing => "containing",
            Self::Eradicating => "eradicating",
            Self::Recovering => "recovering",
            Self::PostIncident => "post_incident",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
        }
    }

    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Resolved | Self::Closed)
    }

    pub fn next_status(&self) -> Option<Self> {
        match self {
            Self::New => Some(Self::Triaging),
            Self::Triaging => Some(Self::Investigating),
            Self::Investigating => Some(Self::Containing),
            Self::Containing => Some(Self::Eradicating),
            Self::Eradicating => Some(Self::Recovering),
            Self::Recovering => Some(Self::PostIncident),
            Self::PostIncident => Some(Self::Resolved),
            Self::Resolved => Some(Self::Closed),
            Self::Closed => None,
        }
    }
}
