use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUploadRequest {
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category_id: Option<String>,
    pub privacy_status: String, // "private", "public", "unlisted"
    pub content_type: String,   // e.g., "video/mp4"
    pub default_language: Option<String>,
    pub default_audio_language: Option<String>,
    pub embeddable: Option<bool>,
    pub license: Option<String>,
    pub public_stats_viewable: Option<bool>,
    pub scheduled_publish_at: Option<String>,
    pub made_for_kids: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityPostRequest {
    pub channel_id: String,
    pub text: String,
    pub attached_video_id: Option<String>,
    pub image_urls: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoListOptions {
    pub channel_id: Option<String>,
    pub for_mine: Option<bool>,
    pub order: Option<String>, // "date", "rating", "relevance", "title", "viewCount"
    pub page_token: Option<String>,
    pub published_after: Option<String>,
    pub published_before: Option<String>,
    pub max_results: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUpdateRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category_id: Option<String>,
    pub privacy_status: Option<String>,
    pub embeddable: Option<bool>,
    pub public_stats_viewable: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistCreateRequest {
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub default_language: Option<String>,
    pub privacy_status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRequest {
    pub channel_id: String,
    pub start_date: String,
    pub end_date: String,
    pub metrics: Option<String>,
    pub dimensions: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBroadcastRequest {
    pub title: String,
    pub description: Option<String>,
    pub scheduled_start_time: String,
    pub privacy_status: String,
    pub enable_auto_start: Option<bool>,
    pub enable_auto_stop: Option<bool>,
    pub enable_dvr: Option<bool>,
    pub enable_embed: Option<bool>,
    pub record_from_start: Option<bool>,
}

// ============================================================================
// API Response Types
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeVideo {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<VideoSnippetResponse>,
    pub content_details: Option<VideoContentDetails>,
    pub statistics: Option<VideoStatistics>,
    pub status: Option<VideoStatusResponse>,
    pub player: Option<VideoPlayer>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSnippetResponse {
    pub title: String,
    pub description: String,
    pub published_at: String,
    pub channel_id: String,
    pub channel_title: String,
    pub thumbnails: Option<Thumbnails>,
    pub tags: Option<Vec<String>>,
    pub category_id: Option<String>,
    pub live_broadcast_content: Option<String>,
    pub default_language: Option<String>,
    pub default_audio_language: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoContentDetails {
    pub duration: String,
    pub dimension: String,
    pub definition: String,
    pub caption: Option<String>,
    pub licensed_content: bool,
    pub projection: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStatistics {
    pub view_count: Option<String>,
    pub like_count: Option<String>,
    pub dislike_count: Option<String>,
    pub favorite_count: Option<String>,
    pub comment_count: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStatusResponse {
    pub upload_status: String,
    pub privacy_status: String,
    pub license: Option<String>,
    pub embeddable: Option<bool>,
    pub public_stats_viewable: Option<bool>,
    pub made_for_kids: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPlayer {
    pub embed_html: Option<String>,
    pub embed_width: Option<i64>,
    pub embed_height: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnails {
    pub default: Option<Thumbnail>,
    pub medium: Option<Thumbnail>,
    pub high: Option<Thumbnail>,
    pub standard: Option<Thumbnail>,
    pub maxres: Option<Thumbnail>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeChannel {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<ChannelSnippet>,
    pub content_details: Option<ChannelContentDetails>,
    pub statistics: Option<ChannelStatistics>,
    pub status: Option<ChannelStatus>,
    pub branding_settings: Option<BrandingSettings>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSnippet {
    pub title: String,
    pub description: String,
    pub custom_url: Option<String>,
    pub published_at: String,
    pub thumbnails: Option<Thumbnails>,
    pub default_language: Option<String>,
    pub country: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelContentDetails {
    pub related_playlists: Option<RelatedPlaylists>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedPlaylists {
    pub likes: Option<String>,
    pub uploads: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStatistics {
    pub view_count: Option<String>,
    pub subscriber_count: Option<String>,
    pub hidden_subscriber_count: bool,
    pub video_count: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStatus {
    pub privacy_status: String,
    pub is_linked: Option<bool>,
    pub long_uploads_status: Option<String>,
    pub made_for_kids: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandingSettings {
    pub channel: Option<ChannelBranding>,
    pub image: Option<ImageBranding>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelBranding {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub default_tab: Option<String>,
    pub country: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBranding {
    pub banner_external_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubePlaylist {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<PlaylistSnippet>,
    pub status: Option<PlaylistStatus>,
    pub content_details: Option<PlaylistContentDetails>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSnippet {
    pub title: String,
    pub description: String,
    pub published_at: String,
    pub channel_id: String,
    pub channel_title: String,
    pub thumbnails: Option<Thumbnails>,
    pub default_language: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistStatus {
    pub privacy_status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistContentDetails {
    pub item_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistItem {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<PlaylistItemSnippet>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistItemSnippet {
    pub playlist_id: String,
    pub position: u32,
    pub resource_id: ResourceId,
    pub title: String,
    pub description: String,
    pub thumbnails: Option<Thumbnails>,
    pub channel_id: String,
    pub channel_title: String,
    pub published_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceId {
    pub kind: String,
    pub video_id: Option<String>,
    pub channel_id: Option<String>,
    pub playlist_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityPost {
    pub id: String,
    pub kind: String,
    pub etag: String,
    pub snippet: Option<CommunityPostSnippet>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityPostSnippet {
    pub channel_id: String,
    pub description: String,
    pub published_at: String,
}
