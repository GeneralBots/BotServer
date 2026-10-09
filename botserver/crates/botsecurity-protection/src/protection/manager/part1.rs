use super::*;

impl std::fmt::Display for ProtectionTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lynis => write!(f, "lynis"),
            Self::RKHunter => write!(f, "rkhunter"),
            Self::Chkrootkit => write!(f, "chkrootkit"),
            Self::Suricata => write!(f, "suricata"),
            Self::LMD => write!(f, "lmd"),
            Self::ClamAV => write!(f, "clamav"),
        }
    }
}
impl std::str::FromStr for ProtectionTool {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lynis" => Ok(Self::Lynis),
            "rkhunter" => Ok(Self::RKHunter),
            "chkrootkit" => Ok(Self::Chkrootkit),
            "suricata" => Ok(Self::Suricata),
            "lmd" | "maldet" => Ok(Self::LMD),
            "clamav" | "clamscan" => Ok(Self::ClamAV),
            _ => Err(()),
        }
    }
}
impl ProtectionTool {

    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::Lynis => "lynis",
            Self::RKHunter => "rkhunter",
            Self::Chkrootkit => "chkrootkit",
            Self::Suricata => "suricata",
            Self::LMD => "maldet",
            Self::ClamAV => "clamscan",
        }
    }

    pub fn service_name(&self) -> Option<&'static str> {
        match self {
            Self::Suricata => Some("suricata"),
            Self::ClamAV => Some("clamav-daemon"),
            _ => None,
        }
    }

    pub fn package_name(&self) -> &'static str {
        match self {
            Self::Lynis => "lynis",
            Self::RKHunter => "rkhunter",
            Self::Chkrootkit => "chkrootkit",
            Self::Suricata => "suricata",
            Self::LMD => "maldetect",
            Self::ClamAV => "clamav",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Lynis,
            Self::RKHunter,
            Self::Chkrootkit,
            Self::Suricata,
            Self::LMD,
            Self::ClamAV,
        ]
    }
}
