use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdCreateRequest {
    pub name: String,
    pub status: String,
    pub creative_id: String,
    pub ad_type: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaUploadRequest {
    pub name: String,
    pub media_type: String, // "VIDEO" or "IMAGE"
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub source_type: String,
    pub retention_in_days: Option<i32>,
}

// ============================================================================
// Response Types
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapchatErrorResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub debug_message: Option<String>,
    pub display_message: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub me: Option<SnapchatUser>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapchatUser {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub email: Option<String>,
    pub organization_id: Option<String>,
    pub display_name: Option<String>,
    pub member_status: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationsResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub organizations: Vec<OrganizationWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationWrapper {
    pub organization: Option<Organization>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub name: String,
    pub address_line_1: Option<String>,
    pub locality: Option<String>,
    pub administrative_district_level_1: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub organization_type: Option<String>,
    pub state: Option<String>,
    pub roles: Option<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdAccountsResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub adaccounts: Vec<AdAccountWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdAccountWrapper {
    pub adaccount: Option<AdAccount>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdAccount {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub name: String,
    pub organization_id: String,
    pub currency: String,
    pub timezone: String,
    pub advertiser: Option<String>,
    pub status: Option<String>,
    pub funding_source_ids: Option<Vec<String>>,
    pub lifetime_spend_cap_micro: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignsResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub campaigns: Vec<CampaignWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignWrapper {
    pub campaign: Option<Campaign>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub name: String,
    pub ad_account_id: String,
    pub status: String,
    pub objective: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub daily_budget_micro: Option<i64>,
    pub lifetime_spend_cap_micro: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdSquadsResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub adsquads: Vec<AdSquadWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdSquadWrapper {
    pub adsquad: Option<AdSquad>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdSquad {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub name: String,
    pub campaign_id: String,
    pub status: String,
    pub squad_type: Option<String>,
    pub billing_event: Option<String>,
    pub bid_micro: Option<i64>,
    pub daily_budget_micro: Option<i64>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub optimization_goal: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativesResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub creatives: Vec<CreativeWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeWrapper {
    pub creative: Option<Creative>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creative {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub name: String,
    pub ad_account_id: String,
    pub creative_type: Option<String>,
    pub headline: Option<String>,
    pub brand_name: Option<String>,
    pub shareable: Option<bool>,
    pub call_to_action: Option<String>,
    pub top_snap_media_id: Option<String>,
    pub top_snap_crop_position: Option<String>,
    pub review_status: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdsResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub ads: Vec<AdWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdWrapper {
    pub ad: Option<Ad>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ad {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub name: String,
    pub ad_squad_id: String,
    pub creative_id: String,
    pub status: String,
    pub ad_type: Option<String>,
    pub review_status: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub media: Vec<MediaWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaWrapper {
    pub media: Option<Media>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleMediaResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub media: Option<Media>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub name: String,
    pub ad_account_id: String,
    pub media_type: String,
    pub media_status: Option<String>,
    pub file_name: Option<String>,
    pub download_link: Option<String>,
    pub duration_secs: Option<f64>,
    pub upload_link: Option<String>,
}
impl Media {
    pub fn is_ready(&self) -> bool {
        self.media_status.as_deref() == Some("READY")
    }

    pub fn is_pending(&self) -> bool {
        self.media_status.as_deref() == Some("PENDING_UPLOAD")
    }

    pub fn is_processing(&self) -> bool {
        self.media_status.as_deref() == Some("PROCESSING")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiencesResponse {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub segments: Vec<AudienceWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceWrapper {
    pub segment: Option<Audience>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audience {
    pub id: String,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub name: String,
    pub ad_account_id: String,
    pub description: Option<String>,
    pub source_type: String,
    pub retention_in_days: Option<i32>,
    pub status: Option<String>,
    pub approximate_number_users: Option<i64>,
    pub upload_status: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignStats {
    pub request_status: Option<String>,
    pub request_id: Option<String>,
    pub total_stats: Option<Vec<StatsEntry>>,
    pub timeseries_stats: Option<Vec<TimeseriesStats>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsEntry {
    pub id: Option<String>,
    pub stats: Option<Stats>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub impressions: Option<i64>,
    pub swipes: Option<i64>,
    pub spend: Option<i64>,
    pub quartile_1: Option<i64>,
    pub quartile_2: Option<i64>,
    pub quartile_3: Option<i64>,
    pub view_completion: Option<i64>,
    pub screen_time_millis: Option<i64>,
    pub video_views: Option<i64>,
    pub video_views_time_based: Option<i64>,
    pub video_views_15s: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesStats {
    pub id: Option<String>,
    pub timeseries: Option<Vec<TimeseriesEntry>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesEntry {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub stats: Option<Stats>,
}
