use super::*;

impl std::fmt::Display for RiskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Mitigated => "mitigated",
            Self::Accepted => "accepted",
            Self::Closed => "closed",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for RiskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "in_progress" => Ok(Self::InProgress),
            "mitigated" => Ok(Self::Mitigated),
            "accepted" => Ok(Self::Accepted),
            "closed" => Ok(Self::Closed),
            _ => Err(format!("Unknown status: {s}")),
        }
    }
}
