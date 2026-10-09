use super::*;

impl std::fmt::Display for RecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(e) => write!(f, "Database error: {e}"),
            Self::NotFound => write!(f, "Recording not found"),
            Self::AlreadyExists => write!(f, "Recording already exists"),
            Self::AlreadyRecording => write!(f, "Already recording"),
            Self::InvalidState(s) => write!(f, "Invalid state: {s}"),
            Self::StorageError(e) => write!(f, "Storage error: {e}"),
            Self::TranscriptionError(e) => write!(f, "Transcription error: {e}"),
            Self::TranscriptionNotReady => write!(f, "Transcription not ready"),
            Self::UnsupportedLanguage(l) => write!(f, "Unsupported language: {l}"),
            Self::ExportFailed(e) => write!(f, "Export failed: {e}"),
            Self::Unauthorized => write!(f, "Unauthorized"),
        }
    }
}
impl std::error::Error for RecordingError {}
