use super::*;

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Exceeded => "exceeded",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Warning => "⚠️",
            Self::Critical => "🚨",
            Self::Exceeded => "🛑",
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            Self::Warning => 1,
            Self::Critical => 2,
            Self::Exceeded => 3,
        }
    }
}
impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
