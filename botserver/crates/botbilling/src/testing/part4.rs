use super::*;

impl std::fmt::Display for MockStripeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::ApiError(code) => write!(f, "API error: {code}"),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Self::AuthenticationError => write!(f, "Authentication failed"),
            Self::RateLimitError => write!(f, "Rate limit exceeded"),
            Self::NetworkError(msg) => write!(f, "Network error: {msg}"),
        }
    }
}
impl std::error::Error for MockStripeError {}
