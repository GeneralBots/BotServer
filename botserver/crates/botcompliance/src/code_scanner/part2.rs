use super::*;

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PasswordInConfig => write!(f, "Password in Config"),
            Self::HardcodedSecret => write!(f, "Hardcoded Secret"),
            Self::DeprecatedKeyword => write!(f, "Deprecated Keyword"),
            Self::FragileCode => write!(f, "Fragile Code"),
            Self::ConfigurationIssue => write!(f, "Configuration Issue"),
            Self::UnderscoreInKeyword => write!(f, "Underscore in Keyword"),
            Self::MissingVault => write!(f, "Missing Vault Config"),
            Self::InsecurePattern => write!(f, "Insecure Pattern"),
            Self::DeprecatedIfInput => write!(f, "Deprecated IF...input Pattern"),
        }
    }
}
