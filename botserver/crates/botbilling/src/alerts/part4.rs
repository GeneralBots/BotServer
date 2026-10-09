use super::*;

impl NotificationChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Webhook => "webhook",
            Self::InApp => "in_app",
            Self::Sms => "sms",
            Self::Slack => "slack",
            Self::MsTeams => "ms_teams",
            Self::Push => "push",
        }
    }
}
