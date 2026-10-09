use super::*;

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Chat => write!(f, "chat"),
            Self::Completion => write!(f, "completion"),
            Self::Embedding => write!(f, "embedding"),
            Self::Rerank => write!(f, "rerank"),
            Self::Moderation => write!(f, "moderation"),
            Self::ImageGeneration => write!(f, "image_generation"),
            Self::AudioTranscription => write!(f, "audio_transcription"),
            Self::AudioGeneration => write!(f, "audio_generation"),
        }
    }
}
