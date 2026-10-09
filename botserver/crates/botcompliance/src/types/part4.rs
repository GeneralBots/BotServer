use super::*;

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Access => "access",
            Self::Modification => "modification",
            Self::Deletion => "deletion",
            Self::Security => "security",
            Self::Admin => "admin",
            Self::Authentication => "authentication",
            Self::Authorization => "authorization",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for AuditEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "access" => Ok(Self::Access),
            "modification" => Ok(Self::Modification),
            "deletion" => Ok(Self::Deletion),
            "security" => Ok(Self::Security),
            "admin" => Ok(Self::Admin),
            "authentication" => Ok(Self::Authentication),
            "authorization" => Ok(Self::Authorization),
            _ => Err(format!("Unknown event type: {s}")),
        }
    }
}
