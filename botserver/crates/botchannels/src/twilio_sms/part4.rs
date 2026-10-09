use super::*;

impl std::fmt::Display for TwilioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(e) => write!(f, "Network error: {}", e),
            Self::ApiError(e) => write!(f, "Twilio API error {}: {}", e.code, e.message),
            Self::ParseError(e) => write!(f, "Parse error: {}", e),
            Self::ConfigError(e) => write!(f, "Configuration error: {}", e),
            Self::InvalidSignature => write!(f, "Invalid webhook signature"),
        }
    }
}
impl std::error::Error for TwilioError {}
pub fn create_twilio_config(
    account_sid: &str,
    auth_token: &str,
    from_number: &str,
) -> TwilioConfig {
    TwilioConfig {
        account_sid: account_sid.to_string(),
        auth_token: auth_token.to_string(),
        from_number: from_number.to_string(),
        status_callback_url: None,
        messaging_service_sid: None,
    }
}
