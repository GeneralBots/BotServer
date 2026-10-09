use super::*;

impl std::fmt::Display for SearchSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Email => write!(f, "email"),
            Self::Drive => write!(f, "drive"),
            Self::Calendar => write!(f, "calendar"),
            Self::Tasks => write!(f, "tasks"),
            Self::Transcription => write!(f, "transcription"),
            Self::Chat => write!(f, "chat"),
            Self::Contacts => write!(f, "contacts"),
            Self::Notes => write!(f, "notes"),
        }
    }
}
