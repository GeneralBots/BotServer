use super::*;

impl std::fmt::Display for OAuthServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CredentialsNotFound => write!(f, "OAuth credentials not found for platform"),
            Self::PlatformNotSupported => write!(f, "Platform does not support OAuth"),
            Self::InvalidState => write!(f, "Invalid OAuth state parameter"),
            Self::StateExpired => write!(f, "OAuth state has expired"),
            Self::TokenNotFound => write!(f, "OAuth token not found"),
            Self::TokenExpired => write!(f, "OAuth token has expired"),
            Self::NoRefreshToken => write!(f, "No refresh token available"),
            Self::AccountNotFound => write!(f, "Connected account not found"),
            Self::NetworkError(msg) => write!(f, "Network error: {msg}"),
            Self::TokenExchangeFailed(msg) => write!(f, "Token exchange failed: {msg}"),
            Self::TokenRefreshFailed(msg) => write!(f, "Token refresh failed: {msg}"),
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}
impl std::error::Error for OAuthServiceError {}
