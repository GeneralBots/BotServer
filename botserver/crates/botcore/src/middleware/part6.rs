use super::*;

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingToken => write!(f, "Missing authorization token"),
            Self::InvalidFormat => write!(f, "Invalid authorization format"),
            Self::InvalidToken(msg) => write!(f, "Invalid token: {msg}"),
            Self::TokenExpired => write!(f, "Token expired"),
        }
    }
}

// ============================================================================
// Response Types
// ============================================================================
