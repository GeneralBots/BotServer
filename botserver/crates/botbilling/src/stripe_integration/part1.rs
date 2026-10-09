use super::*;

impl std::fmt::Display for StripeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiError(e) => write!(f, "Stripe API error: {e}"),
            Self::NetworkError(e) => write!(f, "Network error: {e}"),
            Self::InvalidWebhook(e) => write!(f, "Invalid webhook: {e}"),
            Self::ParseError(e) => write!(f, "Parse error: {e}"),
            Self::NotConfigured => write!(f, "Stripe is not configured"),
        }
    }
}
impl std::error::Error for StripeError {}
