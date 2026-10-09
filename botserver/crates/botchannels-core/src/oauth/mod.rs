pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SocialPlatform {
    Instagram,
    Facebook,
    Twitter,
    LinkedIn,
    Bluesky,
    Threads,
    TikTok,
    YouTube,
    Pinterest,
    Reddit,
    Snapchat,
    Discord,
    WhatsApp,
    Telegram,
    WeChat,
    Twilio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub platform: SocialPlatform,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub platform: SocialPlatform,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    pub id: Uuid,
    pub platform: SocialPlatform,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub redirect_after: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationUrl {
    pub url: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenExchangeRequest {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedAccount {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub platform: SocialPlatform,
    pub account_id: String,
    pub account_name: String,
    pub account_url: Option<String>,
    pub avatar_url: Option<String>,
    pub connected_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub status: AccountStatus,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Expired,
    Revoked,
    Error,
}

pub struct SocialOAuthService {
    credentials: Arc<RwLock<HashMap<SocialPlatform, OAuthCredentials>>>,
    tokens: Arc<RwLock<HashMap<Uuid, OAuthToken>>>,
    states: Arc<RwLock<HashMap<String, OAuthState>>>,
    accounts: Arc<RwLock<HashMap<Uuid, ConnectedAccount>>>,
}

#[derive(Debug, Clone)]
pub enum OAuthServiceError {
    CredentialsNotFound,
    PlatformNotSupported,
    InvalidState,
    StateExpired,
    TokenNotFound,
    TokenExpired,
    NoRefreshToken,
    AccountNotFound,
    NetworkError(String),
    TokenExchangeFailed(String),
    TokenRefreshFailed(String),
    ParseError(String),
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
