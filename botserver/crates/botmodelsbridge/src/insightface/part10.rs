use super::*;

impl std::fmt::Display for InsightFaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "InsightFace service not initialized"),
            Self::InvalidImage(msg) => write!(f, "Invalid image: {msg}"),
            Self::DetectionFailed(msg) => write!(f, "Detection failed: {msg}"),
            Self::IndexNotFound(msg) => write!(f, "Index not found: {msg}"),
            Self::FaceNotFound(msg) => write!(f, "Face not found: {msg}"),
            Self::EmbeddingError(msg) => write!(f, "Embedding error: {msg}"),
            Self::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
            Self::IoError(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}
impl std::error::Error for InsightFaceError {}
