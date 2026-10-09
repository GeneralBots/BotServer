use super::*;

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Required(field) => write!(f, "Field '{}' is required", field),
            Self::TooShort { field, min, actual } => {
                write!(f, "Field '{}' is too short: {} < {} chars", field, actual, min)
            }
            Self::TooLong { field, max, actual } => {
                write!(f, "Field '{}' is too long: {} > {} chars", field, actual, max)
            }
            Self::InvalidFormat { field, expected } => {
                write!(f, "Field '{}' has invalid format, expected: {}", field, expected)
            }
            Self::InvalidRange { field, min, max } => {
                write!(f, "Field '{}' must be between {} and {}", field, min, max)
            }
            Self::InvalidValue { field, message } => {
                write!(f, "Field '{}' has invalid value: {}", field, message)
            }
            Self::InvalidEmail(email) => write!(f, "Invalid email address: {}", email),
            Self::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
            Self::InvalidUuid(uuid) => write!(f, "Invalid UUID: {}", uuid),
            Self::InvalidPhone(phone) => write!(f, "Invalid phone number: {}", phone),
            Self::Forbidden { field, reason } => {
                write!(f, "Field '{}' is forbidden: {}", field, reason)
            }
            Self::Custom(msg) => write!(f, "{}", msg),
        }
    }
}
impl std::error::Error for ValidationError {}
