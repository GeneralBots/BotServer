pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Platform {
    Twitter,
    Facebook,
    Instagram,
    LinkedIn,
    TikTok,
    YouTube,
    Pinterest,
    Snapchat,
    Discord,
    Bluesky,
    Threads,
    WeChat,
    Reddit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaType {
    Image,
    Video,
    Gif,
    Audio,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UploadStatus {
    Pending,
    Uploading,
    Processing,
    Ready,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformLimits {
    pub max_image_size_bytes: u64,
    pub max_video_size_bytes: u64,
    pub max_video_duration_seconds: u32,
    pub supported_image_formats: Vec<String>,
    pub supported_video_formats: Vec<String>,
    pub max_images_per_post: u32,
    pub max_videos_per_post: u32,
    pub image_dimensions: Option<ImageDimensions>,
    pub video_dimensions: Option<VideoDimensions>,
    pub requires_aspect_ratio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub recommended_width: u32,
    pub recommended_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDimensions {
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub min_fps: u32,
    pub max_fps: u32,
    pub min_bitrate_kbps: u32,
    pub max_bitrate_kbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaUpload {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub platform: Platform,
    pub media_type: MediaType,
    pub status: UploadStatus,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub local_path: Option<String>,
    pub platform_media_id: Option<String>,
    pub platform_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub error_message: Option<String>,
    pub upload_progress: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub platform: String,
    pub media_type: Option<String>,
    pub organization_id: Uuid,
    pub alt_text: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkedUploadInit {
    pub platform: String,
    pub organization_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub total_size: u64,
    pub media_type: String,
}

#[derive(Debug, Deserialize)]
pub struct ChunkedUploadAppend {
    pub upload_id: Uuid,
    pub chunk_index: u32,
    pub total_chunks: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChunkedUploadFinalize {
    pub upload_id: Uuid,
    pub alt_text: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: Uuid,
    pub status: String,
    pub platform_media_id: Option<String>,
    pub platform_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ChunkedUploadInitResponse {
    pub upload_id: Uuid,
    pub chunk_size: u64,
    pub total_chunks: u32,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ChunkedUploadAppendResponse {
    pub upload_id: Uuid,
    pub chunks_received: u32,
    pub total_chunks: u32,
    pub progress: f32,
}

#[derive(Debug, Serialize)]
pub struct MediaListResponse {
    pub media: Vec<MediaUpload>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize)]
pub struct MediaListQuery {
    pub platform: Option<String>,
    pub status: Option<String>,
    pub media_type: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub struct ChunkedUploadState {
    pub upload: MediaUpload,
    pub chunks_received: Vec<bool>,
    pub chunk_data: Vec<Vec<u8>>,
    pub chunk_size: u64,
}

pub struct MediaUploadService {
    uploads: Arc<RwLock<HashMap<Uuid, MediaUpload>>>,
    chunked_uploads: Arc<RwLock<HashMap<Uuid, ChunkedUploadState>>>,
    platform_limits: HashMap<Platform, PlatformLimits>,
}

pub(crate) struct PlatformUploadResult {
    media_id: String,
    url: Option<String>,
    thumbnail_url: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MediaUploadError {
    UploadNotFound,
    InvalidChunkIndex,
    ChunkAlreadyReceived,
    UploadExpired,
    FileTooLarge,
    UnsupportedFormat,
    UnsupportedPlatform(String),
    ProcessingError(String),
    StorageError(String),
    ConfigError(String),
    UploadFailed(String),
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
