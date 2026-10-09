use super::*;

impl AuditEventCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Authorization => "authorization",
            Self::DataAccess => "data_access",
            Self::DataModification => "data_modification",
            Self::Administration => "administration",
            Self::Security => "security",
            Self::System => "system",
            Self::Compliance => "compliance",
        }
    }
}
