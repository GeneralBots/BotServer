use super::*;

impl DlpAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Redact => "redact",
            Self::Block => "block",
            Self::Quarantine => "quarantine",
        }
    }
}
