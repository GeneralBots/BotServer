use super::*;

impl From<&str> for SmsPriority {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            "urgent" | "critical" => Self::Urgent,
            _ => Self::Normal,
        }
    }
}
impl std::fmt::Display for SmsPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}
impl From<&str> for SmsProvider {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "twilio" => Self::Twilio,
            "aws" | "aws_sns" | "sns" => Self::AwsSns,
            "vonage" | "nexmo" => Self::Vonage,
            "messagebird" => Self::MessageBird,
            other => Self::Custom(other.to_string()),
        }
    }
}
