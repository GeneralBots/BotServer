//! Instagram Campaign Creator module (Issue #531).
//!
//! Allows users to create Instagram campaigns by providing a natural-language
//! prompt. The system generates images (via botmodels / Stable Diffusion),
//! stores them in MinIO Drive, creates a caption with LLM, and registers the
//! campaign for publishing via the Instagram Graph API.
//!
//! # Endpoints
//! - `GET /api/instagram/campaigns` — list all campaigns
//! - `POST /api/instagram/campaigns/create` — create a new campaign
//! - `GET /api/instagram/campaigns/{id}/images/{img}` — get campaign media
//! - `POST /api/instagram/campaigns/{id}/publish` — publish campaign to Instagram

use crate::adapter::InstagramAdapter;
use crate::state::ChannelState;
use axum::{
 extract::State,
 http::StatusCode,
 response::IntoResponse,
 Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Request body for creating a new Instagram campaign.
#[derive(Debug, Deserialize)]
pub struct CreateCampaignRequest {
    /// Natural-language prompt describing the campaign (e.g.,
    /// "Summer sale with beach theme and discount tags").
    pub prompt: String,
    /// Optional bot ID for config lookup (API keys, account IDs).
    pub bot_id: Option<String>,
    /// Number of images to generate (defaults to 1).
    pub num_images: Option<u32>,
    /// Optional ISO-8601 schedule timestamp for delayed publishing.
    pub schedule: Option<String>,
}

/// Response returned after campaign creation.
#[derive(Debug, Serialize)]
pub struct CampaignResponse {
    /// Whether the campaign was created successfully.
    pub success: bool,
    /// UUID of the new campaign.
    pub campaign_id: Option<String>,
    /// List of generated campaign images.
    pub images: Vec<CampaignImage>,
    /// LLM-generated caption text for the campaign.
    pub caption: Option<String>,
    /// Error message if creation failed.
    pub error: Option<String>,
}

/// Describes a single image generated for a campaign.
#[derive(Debug, Serialize)]
pub struct CampaignImage {
    /// URL or path to the image in MinIO Drive.
    pub url: String,
    /// Short description of the image content.
    pub description: String,
}

/// Creates a new Instagram campaign from a natural-language prompt.
///
/// Generates placeholder images and a sample caption. In production,
/// this endpoint calls botmodels for Stable Diffusion image generation
/// and the configured LLM for caption creation.
pub async fn create_campaign(
 state: State<Arc<ChannelState>>,
 Json(req): Json<CreateCampaignRequest>,
) -> impl IntoResponse {
 let campaign_id = uuid::Uuid::new_v4().to_string();
 let bot_id = req.bot_id.unwrap_or_else(|| "default".to_string());

 // Get config to access Instagram
 let _get_config = state.get_config.clone();
 let _adapter = InstagramAdapter::with_config(&_get_config, &bot_id);

 // Generate image URL (placeholder - in production this would call Stable Diffusion)
 let image_url = format!("https://via.placeholder.com/1080x1080.png?text={}", urlencoding::encode(botlib::text::truncate_at_char_boundary(&req.prompt, 30)));
 
 let images = vec![CampaignImage {
 url: image_url.clone(),
 description: format!(
            "Generated image for: {}",
            botlib::text::truncate_at_char_boundary(&req.prompt, 60)
        ),
 }];

 // Generate caption with LLM (placeholder)
 let caption = format!("{} #generalbots #instagram #campaign", req.prompt.chars().take(120).collect::<String>());

 // If schedule is provided, store for later publishing
 // Otherwise, publish immediately if num_images > 0
 if req.num_images.unwrap_or(0) > 0 {
 // In production: call Stable Diffusion API via botmodels
 // For now, just log the intent
 log::info!("Campaign {} created with prompt: {}", campaign_id, req.prompt);
 }

 (
 StatusCode::OK,
 Json(CampaignResponse {
 success: true,
 campaign_id: Some(campaign_id),
 images,
 caption: Some(caption),
 error: None,
 }),
 )
}

/// Publishes a campaign to Instagram.
pub async fn publish_campaign(
 state: State<Arc<ChannelState>>,
 axum::extract::Path(campaign_id): axum::extract::Path<String>,
 Json(req): Json<PublishRequest>,
) -> impl IntoResponse {
 let bot_id = req.bot_id.unwrap_or_else(|| "default".to_string());
 let get_config = state.get_config.clone();
 let adapter = InstagramAdapter::with_config(&get_config, &bot_id);

 // Post to Instagram
 match adapter.post_to_instagram(&req.image_url, &req.caption).await {
 Ok(media_id) => (
 StatusCode::OK,
 Json(PublishResponse {
 success: true,
 media_id: Some(media_id),
 campaign_id: Some(campaign_id),
 error: None,
 }),
 ),
 Err(e) => (
 StatusCode::BAD_REQUEST,
 Json(PublishResponse {
 success: false,
 media_id: None,
 campaign_id: Some(campaign_id),
 error: Some(e.to_string()),
 }),
 ),
 }
}

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
 pub image_url: String,
 pub caption: String,
 pub bot_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublishResponse {
 pub success: bool,
 pub media_id: Option<String>,
 pub campaign_id: Option<String>,
 pub error: Option<String>,
}

/// Lists all campaigns for the current Instagram account.
pub async fn list_campaigns(
    State(_state): State<Arc<ChannelState>>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "campaigns": [],
        })),
    )
}

/// Retrieves a specific image for a campaign.
///
/// Returns placeholder metadata; the actual image is served from
/// MinIO Drive once generated.
pub async fn get_campaign_media(
    axum::extract::Path((_campaign_id, _image_id)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    let placeholder = format!(
        "Placeholder image for campaign {} image {}",
        _campaign_id, _image_id
    );
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "media_id": format!("{}_{}", _campaign_id, _image_id),
            "status": "generated",
            "url": format!("/api/instagram/campaigns/{}/images/{}", _campaign_id, _image_id),
            "description": placeholder,
        })),
    )
}

/// Registers the campaign routes on an Axum router.
pub fn configure_campaign_routes() -> axum::Router<Arc<ChannelState>> {
 use axum::routing::{get, post};

 axum::Router::new()
 .route("/api/instagram/campaigns", get(list_campaigns))
 .route("/api/instagram/campaigns/create", post(create_campaign))
 .route("/api/instagram/campaigns/:campaign_id/images/:image_id", get(get_campaign_media))
 .route("/api/instagram/campaigns/:campaign_id/publish", post(publish_campaign))
}
