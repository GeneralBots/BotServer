use super::*;

impl SocialPlatform {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Instagram => "Instagram",
            Self::Facebook => "Facebook",
            Self::Twitter => "Twitter / X",
            Self::LinkedIn => "LinkedIn",
            Self::Bluesky => "Bluesky",
            Self::Threads => "Threads",
            Self::TikTok => "TikTok",
            Self::YouTube => "YouTube",
            Self::Pinterest => "Pinterest",
            Self::Reddit => "Reddit",
            Self::Snapchat => "Snapchat",
            Self::Discord => "Discord",
            Self::WhatsApp => "WhatsApp",
            Self::Telegram => "Telegram",
            Self::WeChat => "WeChat",
            Self::Twilio => "Twilio SMS",
        }
    }

    pub fn requires_oauth(&self) -> bool {
        !matches!(self, Self::Bluesky | Self::Telegram | Self::Twilio)
    }

    pub fn authorization_url(&self) -> Option<&'static str> {
        match self {
            Self::Instagram | Self::Facebook | Self::Threads => {
                Some("https://www.facebook.com/v18.0/dialog/oauth")
            }
            Self::Twitter => Some("https://twitter.com/i/oauth2/authorize"),
            Self::LinkedIn => Some("https://www.linkedin.com/oauth/v2/authorization"),
            Self::TikTok => Some("https://www.tiktok.com/v2/auth/authorize/"),
            Self::YouTube => Some("https://accounts.google.com/o/oauth2/v2/auth"),
            Self::Pinterest => Some("https://www.pinterest.com/oauth/"),
            Self::Reddit => Some("https://www.reddit.com/api/v1/authorize"),
            Self::Snapchat => Some("https://accounts.snapchat.com/login/oauth2/authorize"),
            Self::Discord => Some("https://discord.com/api/oauth2/authorize"),
            Self::WeChat => Some("https://open.weixin.qq.com/connect/oauth2/authorize"),
            _ => None,
        }
    }

    pub fn token_url(&self) -> Option<&'static str> {
        match self {
            Self::Instagram | Self::Facebook | Self::Threads => {
                Some("https://graph.facebook.com/v18.0/oauth/access_token")
            }
            Self::Twitter => Some("https://api.twitter.com/2/oauth2/token"),
            Self::LinkedIn => Some("https://www.linkedin.com/oauth/v2/accessToken"),
            Self::TikTok => Some("https://open.tiktokapis.com/v2/oauth/token/"),
            Self::YouTube => Some("https://oauth2.googleapis.com/token"),
            Self::Pinterest => Some("https://api.pinterest.com/v5/oauth/token"),
            Self::Reddit => Some("https://www.reddit.com/api/v1/access_token"),
            Self::Snapchat => Some("https://accounts.snapchat.com/login/oauth2/access_token"),
            Self::Discord => Some("https://discord.com/api/oauth2/token"),
            Self::WeChat => Some("https://api.weixin.qq.com/sns/oauth2/access_token"),
            _ => None,
        }
    }

    pub fn default_scopes(&self) -> Vec<&'static str> {
        match self {
            Self::Instagram => vec![
                "instagram_basic",
                "instagram_content_publish",
                "instagram_manage_comments",
            ],
            Self::Facebook => vec![
                "pages_manage_posts",
                "pages_read_engagement",
                "pages_show_list",
            ],
            Self::Threads => vec!["threads_basic", "threads_content_publish"],
            Self::Twitter => vec!["tweet.read", "tweet.write", "users.read", "offline.access"],
            Self::LinkedIn => vec!["w_organization_social", "r_organization_social"],
            Self::TikTok => vec!["video.upload", "video.publish"],
            Self::YouTube => vec![
                "https://www.googleapis.com/auth/youtube.upload",
                "https://www.googleapis.com/auth/youtube",
            ],
            Self::Pinterest => vec!["boards:read", "pins:read", "pins:write"],
            Self::Reddit => vec!["submit", "read", "identity"],
            Self::Snapchat => vec!["snapchat-marketing-api"],
            Self::Discord => vec!["bot", "applications.commands"],
            Self::WeChat => vec!["snsapi_userinfo"],
            _ => vec![],
        }
    }
}
