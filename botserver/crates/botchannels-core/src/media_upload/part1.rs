use super::*;

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Twitter => "twitter",
            Self::Facebook => "facebook",
            Self::Instagram => "instagram",
            Self::LinkedIn => "linkedin",
            Self::TikTok => "tiktok",
            Self::YouTube => "youtube",
            Self::Pinterest => "pinterest",
            Self::Snapchat => "snapchat",
            Self::Discord => "discord",
            Self::Bluesky => "bluesky",
            Self::Threads => "threads",
            Self::WeChat => "wechat",
            Self::Reddit => "reddit",
        }
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "twitter" | "x" => Some(Self::Twitter),
            "facebook" | "fb" => Some(Self::Facebook),
            "instagram" | "ig" => Some(Self::Instagram),
            "linkedin" => Some(Self::LinkedIn),
            "tiktok" => Some(Self::TikTok),
            "youtube" | "yt" => Some(Self::YouTube),
            "pinterest" => Some(Self::Pinterest),
            "snapchat" | "snap" => Some(Self::Snapchat),
            "discord" => Some(Self::Discord),
            "bluesky" | "bsky" => Some(Self::Bluesky),
            "threads" => Some(Self::Threads),
            "wechat" => Some(Self::WeChat),
            "reddit" => Some(Self::Reddit),
            _ => None,
        }
    }
}
