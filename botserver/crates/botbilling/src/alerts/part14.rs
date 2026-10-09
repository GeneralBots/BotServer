use super::*;

impl std::fmt::Display for AlertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Alert not found"),
            Self::AlreadyExists => write!(f, "Alert already exists"),
            Self::InvalidThreshold => write!(f, "Invalid threshold value"),
            Self::StorageError(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}
impl std::error::Error for AlertError {}
