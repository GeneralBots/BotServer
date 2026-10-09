use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub use botchannels_core::{media_upload, oauth};

pub mod analytics;
pub mod handoff;
pub mod survey;
pub mod bluesky;
pub mod discord;
pub mod facebook;
pub mod instagram_channel;
pub mod linkedin;
pub mod pinterest;
pub mod reddit;
pub mod snapchat;
pub mod telegram_channel;
pub mod threads;
pub mod tiktok;
pub mod twilio_sms;
pub mod twitter;
pub mod wechat;
pub mod youtube;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Bluesky,
    Discord,
    Facebook,
    Instagram,
    LinkedIn,
    Pinterest,
    Reddit,
    Snapchat,
    Telegram,
    Threads,
    TikTok,
    TwilioSms,
    Twitter,
    WeChat,
    WhatsApp,
    YouTube,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAccount {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub channel_type: ChannelType,
    pub credentials: ChannelCredentials,
    pub settings: ChannelSettings,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelCredentials {
    OAuth {
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        scope: Option<String>,
    },
    ApiKey {
        api_key: String,
        api_secret: Option<String>,
    },
    UsernamePassword {
        username: String,
        password: String,
        app_password: Option<String>,
    },
    Twilio {
        account_sid: String,
        auth_token: String,
        from_number: String,
    },
    Custom {
        data: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelSettings {
    pub default_hashtags: Vec<String>,
    pub auto_shorten_links: bool,
    pub schedule_enabled: bool,
    pub timezone: Option<String>,
    pub rate_limit_per_hour: Option<u32>,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostContent {
    pub text: Option<String>,
    pub image_urls: Vec<String>,
    pub video_url: Option<String>,
    pub link: Option<String>,
    pub hashtags: Vec<String>,
    pub mentions: Vec<String>,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResult {
    pub success: bool,
    pub channel_type: ChannelType,
    pub post_id: Option<String>,
    pub url: Option<String>,
    pub error: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum ChannelError {
    UnknownChannel(String),
    AccountNotFound(String),
    AuthenticationFailed(String),
    RateLimited { retry_after: Option<u64> },
    ContentTooLong { max_length: usize, actual_length: usize },
    UnsupportedMediaType(String),
    NetworkError(String),
    ApiError { code: Option<String>, message: String },
    NotConfigured,
}

#[async_trait::async_trait]
pub trait ChannelProvider: Send + Sync {
    fn channel_type(&self) -> ChannelType;
    fn max_text_length(&self) -> usize;
    fn supports_images(&self) -> bool;
    fn supports_video(&self) -> bool;
    fn supports_links(&self) -> bool;

    async fn post(&self, account: &ChannelAccount, content: &PostContent) -> Result<PostResult, ChannelError>;
    async fn validate_credentials(&self, credentials: &ChannelCredentials) -> Result<bool, ChannelError>;
    async fn refresh_token(&self, account: &mut ChannelAccount) -> Result<(), ChannelError>;
}

pub struct ChannelManager {
    accounts: Arc<RwLock<HashMap<String, ChannelAccount>>>,
    providers: HashMap<ChannelType, Arc<dyn ChannelProvider>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelLimits {
    pub max_text_length: usize,
    pub supports_images: bool,
    pub supports_video: bool,
    pub supports_links: bool,
}

pub struct MultiPostRequest {
    pub content: PostContent,
    pub targets: Vec<PostTarget>,
}

#[derive(Debug, Clone)]
pub enum PostTarget {
    Account(String),
    Channel(ChannelType),
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPostResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<PostResult>,
}

mod lib_part1;
mod lib_part2;
mod lib_part3;
mod lib_part4;
mod lib_part5;
mod lib_part6;
