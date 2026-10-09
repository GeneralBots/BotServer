use super::*;

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownChannel(name) => write!(f, "Unknown channel: {name}"),
            Self::AccountNotFound(name) => write!(f, "Account not found: {name}"),
            Self::AuthenticationFailed(msg) => write!(f, "Authentication failed: {msg}"),
            Self::RateLimited { retry_after } => {
                if let Some(secs) = retry_after {
                    write!(f, "Rate limited, retry after {secs} seconds")
                } else {
                    write!(f, "Rate limited")
                }
            }
            Self::ContentTooLong { max_length, actual_length } => {
                write!(f, "Content too long: {actual_length} characters (max: {max_length})")
            }
            Self::UnsupportedMediaType(media_type) => {
                write!(f, "Unsupported media type: {media_type}")
            }
            Self::NetworkError(msg) => write!(f, "Network error: {msg}"),
            Self::ApiError { code, message } => {
                if let Some(c) = code {
                    write!(f, "API error [{c}]: {message}")
                } else {
                    write!(f, "API error: {message}")
                }
            }
            Self::NotConfigured => write!(f, "Channel not configured"),
        }
    }
}
impl std::error::Error for ChannelError {}
