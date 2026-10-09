use super::*;

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::VerificationFailed(msg) => write!(f, "Verification failed: {msg}"),
            Self::StorageError(msg) => write!(f, "Storage error: {msg}"),
            Self::EncryptionError(msg) => write!(f, "Encryption error: {msg}"),
            Self::PolicyViolation(msg) => write!(f, "Policy violation: {msg}"),
            Self::RestoreFailed(msg) => write!(f, "Restore failed: {msg}"),
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {msg}"),
        }
    }
}
impl std::error::Error for BackupError {}
