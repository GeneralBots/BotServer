use super::*;

impl From<String> for TriggerType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "tool" => Self::Tool,
            "schedule" => Self::Schedule,
            "event" => Self::Event,
            "always" => Self::Always,
            _ => Self::Keyword,
        }
    }
}
impl fmt::Display for TriggerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keyword => write!(f, "keyword"),
            Self::Tool => write!(f, "tool"),
            Self::Schedule => write!(f, "schedule"),
            Self::Event => write!(f, "event"),
            Self::Always => write!(f, "always"),
        }
    }
}
