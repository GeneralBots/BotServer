use super::*;

impl std::fmt::Display for RiskCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Technical => "technical",
            Self::Operational => "operational",
            Self::Financial => "financial",
            Self::Compliance => "compliance",
            Self::Reputational => "reputational",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for RiskCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "technical" => Ok(Self::Technical),
            "operational" => Ok(Self::Operational),
            "financial" => Ok(Self::Financial),
            "compliance" => Ok(Self::Compliance),
            "reputational" => Ok(Self::Reputational),
            _ => Err(format!("Unknown category: {s}")),
        }
    }
}
