use super::*;

impl std::fmt::Display for ComplianceFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Gdpr => "gdpr",
            Self::Soc2 => "soc2",
            Self::Iso27001 => "iso27001",
            Self::Hipaa => "hipaa",
            Self::PciDss => "pci_dss",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for ComplianceFramework {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gdpr" => Ok(Self::Gdpr),
            "soc2" => Ok(Self::Soc2),
            "iso27001" => Ok(Self::Iso27001),
            "hipaa" => Ok(Self::Hipaa),
            "pci_dss" | "pcidss" => Ok(Self::PciDss),
            _ => Err(format!("Unknown framework: {s}")),
        }
    }
}
