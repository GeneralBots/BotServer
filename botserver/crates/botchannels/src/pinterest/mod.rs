//! Pinterest API v5 Integration
//!
//! Provides Pin creation, Board management, and Advertising capabilities.
//! Supports OAuth 2.0 authentication flow for Pinterest API.

pub(crate) use crate::{ ChannelAccount, ChannelCredentials, ChannelError, ChannelProvider, ChannelType, PostContent, PostResult, };
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;

/// Pinterest API v5 provider for pins, boards, and ads
pub struct PinterestProvider {
    client: reqwest::Client,
}

    /// Provider endpoints are compile-time constants: tokens are scoped to
    /// this host and must never be sent to a runtime-configured one.
    const API_BASE: &str = "https://api.pinterest.com/v5";
    const OAUTH_BASE: &str = "https://api.pinterest.com/v5/oauth";

    /// Delete a board

    /// List campaigns

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinCreateRequest {
    pub board_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_section_id: Option<String>,
    pub media_source: MediaSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum MediaSource {
    #[serde(rename = "image_url")]
    ImageUrl { url: String },
    #[serde(rename = "video_id")]
    VideoId {
        cover_image_url: String,
        media_id: String,
    },
    #[serde(rename = "multiple_image_urls")]
    MultipleImageUrls { items: Vec<ImageItem> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageItem {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardCreateRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy: Option<String>, // "PUBLIC" or "SECRET"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSectionCreateRequest {
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct BoardListOptions {
    pub page_size: Option<u32>,
    pub bookmark: Option<String>,
    pub privacy: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PinListOptions {
    pub page_size: Option<u32>,
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PaginationOptions {
    pub page_size: Option<u32>,
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub page_size: Option<u32>,
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CampaignListOptions {
    pub page_size: Option<u32>,
    pub bookmark: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignCreateRequest {
    pub ad_account_id: String,
    pub name: String,
    pub status: String,
    pub objective_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_spend_cap: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifetime_spend_cap: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdGroupCreateRequest {
    pub ad_account_id: String,
    pub campaign_id: String,
    pub name: String,
    pub status: String,
    pub budget_in_micro_currency: i64,
    pub bid_in_micro_currency: Option<i64>,
    pub optimization_goal_metadata: Option<serde_json::Value>,
    pub targeting_spec: Option<TargetingSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetingSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_bucket: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interest: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdCreateRequest {
    pub ad_group_id: String,
    pub creative_type: String,
    pub pin_id: String,
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking_urls: Option<TrackingUrls>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingUrls {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impression: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engagement: Option<Vec<String>>,
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinterestErrorResponse {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinterestUser {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_section_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<PinMedia>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinMedia {
    pub media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<HashMap<String, ImageInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follower_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<BoardOwner>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardOwner {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSection {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardListResponse {
    pub items: Vec<Board>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinListResponse {
    pub items: Vec<Pin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSectionListResponse {
    pub items: Vec<BoardSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinSearchResponse {
    pub items: Vec<Pin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdAccount {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub country: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<AdAccountOwner>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdAccountOwner {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdAccountListResponse {
    pub items: Vec<AdAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub ad_account_id: String,
    pub name: String,
    pub status: String,
    pub objective_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_spend_cap: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifetime_spend_cap: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignListResponse {
    pub items: Vec<Campaign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdGroup {
    pub id: String,
    pub ad_account_id: String,
    pub campaign_id: String,
    pub name: String,
    pub status: String,
    pub budget_in_micro_currency: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_in_micro_currency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ad {
    pub id: String,
    pub ad_group_id: String,
    pub creative_type: String,
    pub pin_id: String,
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinAnalytics {
    pub all: Option<AnalyticsData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAnalytics {
    pub all: Option<AnalyticsData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsData {
    pub daily_metrics: Option<Vec<DailyMetric>>,
    pub summary_metrics: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyMetric {
    pub date: String,
    pub data_status: String,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
}

// ============================================================================
// Constants
// ============================================================================

/// Board privacy options
pub struct BoardPrivacy;

/// Campaign objective types
pub struct CampaignObjective;

/// Ad status values
pub struct AdStatus;

/// Creative types
pub struct CreativeType;

/// Analytics metric types
pub struct AnalyticsMetrics;

/// OAuth scopes
pub struct PinterestScopes;

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
mod part12;
