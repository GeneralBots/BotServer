use super::*;

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bluesky => write!(f, "bluesky"),
            Self::Discord => write!(f, "discord"),
            Self::Facebook => write!(f, "facebook"),
            Self::Instagram => write!(f, "instagram"),
            Self::LinkedIn => write!(f, "linkedin"),
            Self::Pinterest => write!(f, "pinterest"),
            Self::Reddit => write!(f, "reddit"),
            Self::Snapchat => write!(f, "snapchat"),
            Self::Telegram => write!(f, "telegram"),
            Self::Threads => write!(f, "threads"),
            Self::TikTok => write!(f, "tiktok"),
            Self::TwilioSms => write!(f, "twilio_sms"),
            Self::Twitter => write!(f, "twitter"),
            Self::WeChat => write!(f, "wechat"),
            Self::WhatsApp => write!(f, "whatsapp"),
            Self::YouTube => write!(f, "youtube"),
        }
    }
}
impl std::str::FromStr for ChannelType {
    type Err = ChannelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bluesky" => Ok(Self::Bluesky),
            "discord" => Ok(Self::Discord),
            "facebook" | "fb" => Ok(Self::Facebook),
            "instagram" | "ig" => Ok(Self::Instagram),
            "linkedin" => Ok(Self::LinkedIn),
            "pinterest" => Ok(Self::Pinterest),
            "reddit" => Ok(Self::Reddit),
            "snapchat" => Ok(Self::Snapchat),
            "telegram" | "tg" => Ok(Self::Telegram),
            "threads" => Ok(Self::Threads),
            "tiktok" => Ok(Self::TikTok),
            "twilio" | "twilio_sms" | "sms" => Ok(Self::TwilioSms),
            "twitter" | "x" => Ok(Self::Twitter),
            "wechat" => Ok(Self::WeChat),
            "whatsapp" | "wa" => Ok(Self::WhatsApp),
            "youtube" | "yt" => Ok(Self::YouTube),
            _ => Err(ChannelError::UnknownChannel(s.to_string())),
        }
    }
}
