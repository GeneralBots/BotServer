use super::*;

impl std::fmt::Display for MediaUploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UploadNotFound => write!(f, "Upload not found"),
            Self::InvalidChunkIndex => write!(f, "Invalid chunk index"),
            Self::ChunkAlreadyReceived => write!(f, "Chunk already received"),
            Self::UploadExpired => write!(f, "Upload expired"),
            Self::FileTooLarge => write!(f, "File too large"),
            Self::UnsupportedFormat => write!(f, "Unsupported format"),
            Self::UnsupportedPlatform(p) => write!(f, "Unsupported platform: {p}"),
            Self::ProcessingError(e) => write!(f, "Processing error: {e}"),
            Self::StorageError(e) => write!(f, "Storage error: {e}"),
            Self::ConfigError(e) => write!(f, "Configuration error: {e}"),
            Self::UploadFailed(e) => write!(f, "Upload failed: {e}"),
        }
    }
}
impl std::error::Error for MediaUploadError {}
