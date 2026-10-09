use super::*;

impl std::fmt::Display for ApprovalChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Email => write!(f, "email"),
            Self::Sms => write!(f, "sms"),
            Self::Mobile => write!(f, "mobile"),
            Self::Teams => write!(f, "teams"),
            Self::Slack => write!(f, "slack"),
            Self::Webhook => write!(f, "webhook"),
            Self::InApp => write!(f, "in_app"),
        }
    }
}
