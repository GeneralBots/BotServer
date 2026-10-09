//! TikTok Content Posting API Integration
//!
//! Provides video upload, publishing, and user management capabilities.
//! Supports OAuth 2.0 authentication flow for TikTok Login Kit.

pub(crate) use crate::{ ChannelAccount, ChannelCredentials, ChannelError, ChannelProvider, ChannelType, PostContent, PostResult, };
pub(crate) use serde::{Deserialize, Serialize};

/// TikTok API provider for video uploads and content posting
pub struct TikTokProvider {
    client: reqwest::Client,
    api_base_url: String,
    oauth_base_url: String,
}

    /// Query specific videos by IDs (requires video.list scope)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUploadRequest {
    pub title: String,
    pub privacy_level: String, // "PUBLIC_TO_EVERYONE", "MUTUAL_FOLLOW_FRIENDS", "SELF_ONLY", "FOLLOWER_OF_CREATOR"
    pub video_url: Option<String>,
    pub disable_duet: Option<bool>,
    pub disable_comment: Option<bool>,
    pub disable_stitch: Option<bool>,
    pub video_cover_timestamp_ms: Option<u64>,
    pub brand_content_toggle: Option<bool>,
    pub brand_organic_toggle: Option<bool>,
    // For FILE_UPLOAD source
    pub video_size: Option<u64>,
    pub chunk_size: Option<u64>,
    pub total_chunk_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoPostRequest {
    pub title: String,
    pub description: Option<String>,
    pub privacy_level: String,
    pub photo_urls: Vec<String>,
    pub photo_cover_index: Option<u32>,
    pub disable_comment: Option<bool>,
    pub auto_add_music: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoListOptions {
    pub cursor: Option<i64>,
    pub max_count: Option<u32>,
    pub fields: Option<String>,
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokApiResponse<T> {
    pub data: Option<T>,
    pub error: Option<TikTokError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokError {
    pub code: String,
    pub message: String,
    pub log_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUploadInit {
    pub publish_id: String,
    pub upload_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoUploadInit {
    pub publish_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishStatus {
    pub status: String, // "PROCESSING_UPLOAD", "PROCESSING_DOWNLOAD", "SEND_TO_USER_INBOX", "PUBLISH_COMPLETE", "FAILED"
    #[serde(default)]
    pub fail_reason: Option<String>,
    pub publicly_available_post_id: Option<Vec<String>>,
    pub uploaded_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfoData {
    pub user: Option<TikTokUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokUser {
    pub open_id: String,
    pub union_id: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub avatar_url_100: Option<String>,
    pub avatar_large_url: Option<String>,
    pub bio_description: Option<String>,
    pub profile_deep_link: Option<String>,
    pub is_verified: Option<bool>,
    pub follower_count: Option<u64>,
    pub following_count: Option<u64>,
    pub likes_count: Option<u64>,
    pub video_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoListResponse {
    pub videos: Vec<TikTokVideo>,
    pub cursor: Option<i64>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoQueryResponse {
    pub videos: Vec<TikTokVideo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokVideo {
    pub id: String,
    pub create_time: Option<i64>,
    pub cover_image_url: Option<String>,
    pub share_url: Option<String>,
    pub video_description: Option<String>,
    pub duration: Option<u32>,
    pub title: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub like_count: Option<u64>,
    pub comment_count: Option<u64>,
    pub share_count: Option<u64>,
    pub view_count: Option<u64>,
    pub embed_html: Option<String>,
    pub embed_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorInfo {
    pub creator_avatar_url: Option<String>,
    pub creator_username: Option<String>,
    pub creator_nickname: Option<String>,
    pub privacy_level_options: Option<Vec<String>>,
    pub comment_disabled: Option<bool>,
    pub duet_disabled: Option<bool>,
    pub stitch_disabled: Option<bool>,
    pub max_video_post_duration_sec: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub refresh_expires_in: u64,
    pub open_id: String,
    pub scope: String,
    pub token_type: String,
}

// ============================================================================
// Privacy Level Constants
// ============================================================================

/// Privacy levels for TikTok posts
pub struct PrivacyLevel;

/// OAuth scopes for TikTok API
pub struct TikTokScopes;

/// Supported video formats for TikTok
pub struct VideoFormats;

/// Supported image formats for TikTok photo posts
pub struct ImageFormats;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;
mod part11;

pub use part9::*;
