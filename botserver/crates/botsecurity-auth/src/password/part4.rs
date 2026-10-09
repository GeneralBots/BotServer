use super::*;

impl PasswordIssue {
    pub fn message(&self) -> String {
        match self {
            Self::TooShort { min, actual } => {
                format!("Password must be at least {min} characters (currently {actual})")
            }
            Self::TooLong { max, actual } => {
                format!("Password must be at most {max} characters (currently {actual})")
            }
            Self::MissingUppercase => "Password must contain at least one uppercase letter".into(),
            Self::MissingLowercase => "Password must contain at least one lowercase letter".into(),
            Self::MissingDigit => "Password must contain at least one digit".into(),
            Self::MissingSpecial => "Password must contain at least one special character".into(),
            Self::InsufficientUniqueChars { min, actual } => {
                format!("Password must have at least {min} unique characters (currently {actual})")
            }
            Self::TooManyConsecutiveChars { max } => {
                format!("Password must not have more than {max} consecutive identical characters")
            }
            Self::CommonPassword => "This password is too common and easily guessed".into(),
            Self::ContainsUsername => "Password must not contain your username".into(),
            Self::ContainsEmail => "Password must not contain your email address".into(),
            Self::RecentlyUsed => "This password was used recently, please choose a new one".into(),
            Self::Compromised => "This password has been found in data breaches".into(),
        }
    }
}
