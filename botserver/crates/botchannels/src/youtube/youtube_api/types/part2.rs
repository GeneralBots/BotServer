use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentThread {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<CommentThreadSnippet>,
    pub replies: Option<CommentReplies>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentThreadSnippet {
    pub channel_id: String,
    pub video_id: String,
    pub top_level_comment: Comment,
    pub can_reply: bool,
    pub total_reply_count: u32,
    pub is_public: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<CommentSnippet>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentSnippet {
    pub video_id: Option<String>,
    pub text_display: String,
    pub text_original: String,
    pub author_display_name: String,
    pub author_profile_image_url: Option<String>,
    pub author_channel_url: Option<String>,
    pub author_channel_id: Option<AuthorChannelId>,
    pub can_rate: bool,
    pub viewer_rating: Option<String>,
    pub like_count: u32,
    pub published_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorChannelId {
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentReplies {
    pub comments: Vec<Comment>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<SubscriptionSnippet>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionSnippet {
    pub published_at: String,
    pub title: String,
    pub description: String,
    pub resource_id: ResourceId,
    pub channel_id: String,
    pub thumbnails: Option<Thumbnails>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveBroadcast {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<LiveBroadcastSnippet>,
    pub status: Option<LiveBroadcastStatus>,
    pub content_details: Option<LiveBroadcastContentDetails>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveBroadcastSnippet {
    pub published_at: String,
    pub channel_id: String,
    pub title: String,
    pub description: String,
    pub thumbnails: Option<Thumbnails>,
    pub scheduled_start_time: Option<String>,
    pub scheduled_end_time: Option<String>,
    pub actual_start_time: Option<String>,
    pub actual_end_time: Option<String>,
    pub is_default_broadcast: bool,
    pub live_chat_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveBroadcastStatus {
    pub life_cycle_status: String,
    pub privacy_status: String,
    pub recording_status: Option<String>,
    pub made_for_kids: Option<bool>,
    pub self_declared_made_for_kids: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveBroadcastContentDetails {
    pub bound_stream_id: Option<String>,
    pub bound_stream_last_update_time_ms: Option<String>,
    pub enable_closed_captions: Option<bool>,
    pub enable_content_encryption: Option<bool>,
    pub enable_dvr: Option<bool>,
    pub enable_embed: Option<bool>,
    pub enable_auto_start: Option<bool>,
    pub enable_auto_stop: Option<bool>,
    pub record_from_start: Option<bool>,
    pub start_with_slate: Option<bool>,
    pub projection: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailSetResponse {
    pub kind: String,
    pub etag: String,
    pub items: Vec<ThumbnailItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailItem {
    pub default: Option<Thumbnail>,
    pub medium: Option<Thumbnail>,
    pub high: Option<Thumbnail>,
    pub standard: Option<Thumbnail>,
    pub maxres: Option<Thumbnail>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsResponse {
    pub kind: String,
    pub column_headers: Vec<ColumnHeader>,
    pub rows: Option<Vec<Vec<serde_json::Value>>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnHeader {
    pub name: String,
    pub column_type: String,
    pub data_type: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
    pub scope: Option<String>,
}

// ============================================================================
// List Response Types
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelListResponse {
    pub kind: String,
    pub etag: String,
    pub page_info: Option<PageInfo>,
    pub items: Vec<YouTubeChannel>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeVideoListResponse {
    pub kind: String,
    pub etag: String,
    pub page_info: Option<PageInfo>,
    pub items: Vec<YouTubeVideo>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoListResponse {
    pub kind: String,
    pub etag: String,
    pub next_page_token: Option<String>,
    pub prev_page_token: Option<String>,
    pub page_info: Option<PageInfo>,
    pub items: Vec<VideoSearchResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSearchResult {
    pub kind: String,
    pub etag: String,
    pub id: VideoSearchId,
    pub snippet: Option<VideoSnippetResponse>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSearchId {
    pub kind: String,
    pub video_id: Option<String>,
    pub channel_id: Option<String>,
    pub playlist_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentThreadListResponse {
    pub kind: String,
    pub etag: String,
    pub next_page_token: Option<String>,
    pub page_info: Option<PageInfo>,
    pub items: Vec<CommentThread>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub total_results: u32,
    pub results_per_page: u32,
}

// ============================================================================
// Helper Functions and Constants
// ============================================================================
impl YouTubeVideo {
    /// Get the video URL
    pub fn url(&self) -> String {
        format!("https://www.youtube.com/watch?v={}", self.id)
    }

    /// Get the embed URL
    pub fn embed_url(&self) -> String {
        format!("https://www.youtube.com/embed/{}", self.id)
    }

    /// Get the thumbnail URL (high quality)
    pub fn thumbnail_url(&self) -> Option<String> {
        self.snippet
            .as_ref()
            .and_then(|s| s.thumbnails.as_ref())
            .and_then(|t| {
                t.high
                    .as_ref()
                    .or(t.medium.as_ref())
                    .or(t.default.as_ref())
            })
            .map(|t| t.url.clone())
    }
}
impl YouTubeChannel {
    /// Get the channel URL
    pub fn url(&self) -> String {
        if let Some(snippet) = &self.snippet {
            if let Some(custom_url) = &snippet.custom_url {
                return format!("https://www.youtube.com/{}", custom_url);
            }
        }
        format!("https://www.youtube.com/channel/{}", self.id)
    }
}
/// Video categories commonly used on YouTube
pub struct VideoCategories;
impl VideoCategories {
    pub const FILM_AND_ANIMATION: &'static str = "1";
    pub const AUTOS_AND_VEHICLES: &'static str = "2";
    pub const MUSIC: &'static str = "10";
    pub const PETS_AND_ANIMALS: &'static str = "15";
    pub const SPORTS: &'static str = "17";
    pub const TRAVEL_AND_EVENTS: &'static str = "19";
    pub const GAMING: &'static str = "20";
    pub const PEOPLE_AND_BLOGS: &'static str = "22";
    pub const COMEDY: &'static str = "23";
    pub const ENTERTAINMENT: &'static str = "24";
    pub const NEWS_AND_POLITICS: &'static str = "25";
    pub const HOWTO_AND_STYLE: &'static str = "26";
    pub const EDUCATION: &'static str = "27";
    pub const SCIENCE_AND_TECHNOLOGY: &'static str = "28";
    pub const NONPROFITS_AND_ACTIVISM: &'static str = "29";
}
/// Privacy status options for videos and playlists
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyStatus {
    Public,
    Private,
    Unlisted,
}
impl PrivacyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Unlisted => "unlisted",
        }
    }
}
impl std::fmt::Display for PrivacyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
