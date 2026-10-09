use super::*;

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}
