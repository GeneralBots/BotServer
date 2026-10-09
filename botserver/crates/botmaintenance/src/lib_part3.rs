use super::*;

impl std::fmt::Display for CleanupTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manual => write!(f, "Manual"),
            Self::Scheduled => write!(f, "Scheduled"),
            Self::StorageAlert => write!(f, "Storage Alert"),
        }
    }
}
