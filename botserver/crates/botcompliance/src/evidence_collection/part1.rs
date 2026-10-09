use super::*;

impl std::fmt::Display for CollectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::NotAutomated(msg) => write!(f, "Not automated: {msg}"),
            Self::ValidationFailed(msg) => write!(f, "Validation failed: {msg}"),
            Self::StorageError(msg) => write!(f, "Storage error: {msg}"),
            Self::SourceError(msg) => write!(f, "Source error: {msg}"),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}
impl std::error::Error for CollectionError {}
