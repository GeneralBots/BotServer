use super::*;

impl Default for ConstraintType {
    fn default() -> Self {
        Self::Custom("unknown".to_string())
    }
}
impl std::fmt::Display for ConstraintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Budget => write!(f, "budget"),
            Self::Permission => write!(f, "permission"),
            Self::Policy => write!(f, "policy"),
            Self::Compliance => write!(f, "compliance"),
            Self::Technical => write!(f, "technical"),
            Self::RateLimit => write!(f, "rate_limit"),
            Self::TimeWindow => write!(f, "time_window"),
            Self::DataAccess => write!(f, "data_access"),
            Self::Security => write!(f, "security"),
            Self::Resource => write!(f, "resource"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}
