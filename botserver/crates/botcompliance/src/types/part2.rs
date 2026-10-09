use super::*;

impl std::fmt::Display for ComplianceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Compliant => "compliant",
            Self::PartialCompliance => "partial_compliance",
            Self::NonCompliant => "non_compliant",
            Self::InProgress => "in_progress",
            Self::NotApplicable => "not_applicable",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for ComplianceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "compliant" => Ok(Self::Compliant),
            "partial_compliance" => Ok(Self::PartialCompliance),
            "non_compliant" => Ok(Self::NonCompliant),
            "in_progress" => Ok(Self::InProgress),
            "not_applicable" => Ok(Self::NotApplicable),
            _ => Err(format!("Unknown status: {s}")),
        }
    }
}
