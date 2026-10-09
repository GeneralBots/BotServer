use super::*;

impl SignatureValidation {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    pub fn error_message(&self) -> &'static str {
        match self {
            Self::Valid => "Valid",
            Self::Missing => "Signature header missing",
            Self::Invalid => "Invalid signature",
            Self::Expired => "Timestamp expired",
            Self::Replayed => "Duplicate request detected",
        }
    }
}
