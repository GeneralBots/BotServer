use super::*;

impl std::fmt::Display for TreatmentStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Mitigate => "mitigate",
            Self::Accept => "accept",
            Self::Transfer => "transfer",
            Self::Avoid => "avoid",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for TreatmentStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mitigate" => Ok(Self::Mitigate),
            "accept" => Ok(Self::Accept),
            "transfer" => Ok(Self::Transfer),
            "avoid" => Ok(Self::Avoid),
            _ => Err(format!("Unknown strategy: {s}")),
        }
    }
}
