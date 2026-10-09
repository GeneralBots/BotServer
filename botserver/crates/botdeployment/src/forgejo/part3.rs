use super::*;

impl std::fmt::Display for ForgejoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForgejoError::HttpError(msg) => write!(f, "HTTP error: {}", msg),
            ForgejoError::JsonError(msg) => write!(f, "JSON error: {}", msg),
            ForgejoError::ApiError(msg) => write!(f, "API error: {}", msg),
            ForgejoError::GitError(msg) => write!(f, "Git error: {}", msg),
        }
    }
}
impl std::error::Error for ForgejoError {}
