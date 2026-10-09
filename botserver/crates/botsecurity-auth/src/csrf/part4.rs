use super::*;

impl CsrfValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    pub fn error_message(&self) -> &'static str {
        match self {
            Self::Valid => "Valid",
            Self::Missing => "CSRF token missing",
            Self::Invalid => "CSRF token invalid",
            Self::Expired => "CSRF token expired",
            Self::SessionMismatch => "CSRF token session mismatch",
        }
    }
}
