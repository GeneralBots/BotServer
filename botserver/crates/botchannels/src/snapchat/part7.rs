use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub scope: String,
}

// ============================================================================
// Constants
// ============================================================================
/// Campaign objectives
pub struct CampaignObjectives;
impl CampaignObjectives {
    pub const AWARENESS: &'static str = "AWARENESS";
    pub const APP_INSTALLS: &'static str = "APP_INSTALLS";
    pub const ENGAGEMENT: &'static str = "ENGAGEMENT";
    pub const VIDEO_VIEWS: &'static str = "VIDEO_VIEWS";
    pub const WEB_CONVERSIONS: &'static str = "WEB_CONVERSIONS";
    pub const LEAD_GENERATION: &'static str = "LEAD_GENERATION";
    pub const CATALOG_SALES: &'static str = "CATALOG_SALES";
}
/// Ad status values
pub struct AdStatus;
impl AdStatus {
    pub const ACTIVE: &'static str = "ACTIVE";
    pub const PAUSED: &'static str = "PAUSED";
}
/// Billing events
pub struct BillingEvents;
impl BillingEvents {
    pub const IMPRESSION: &'static str = "IMPRESSION";
    pub const SWIPE: &'static str = "SWIPE";
    pub const VIDEO_VIEW: &'static str = "VIDEO_VIEW";
}
/// Optimization goals
pub struct OptimizationGoals;
impl OptimizationGoals {
    pub const IMPRESSIONS: &'static str = "IMPRESSIONS";
    pub const SWIPES: &'static str = "SWIPES";
    pub const APP_INSTALLS: &'static str = "APP_INSTALLS";
    pub const VIDEO_VIEWS: &'static str = "VIDEO_VIEWS";
    pub const VIDEO_VIEWS_15_SEC: &'static str = "VIDEO_VIEWS_15_SEC";
    pub const USES: &'static str = "USES";
    pub const STORY_OPENS: &'static str = "STORY_OPENS";
    pub const PIXEL_PAGE_VIEW: &'static str = "PIXEL_PAGE_VIEW";
    pub const PIXEL_ADD_TO_CART: &'static str = "PIXEL_ADD_TO_CART";
    pub const PIXEL_PURCHASE: &'static str = "PIXEL_PURCHASE";
    pub const PIXEL_SIGNUP: &'static str = "PIXEL_SIGNUP";
}
/// Call to action types
pub struct CallToAction;
impl CallToAction {
    pub const APPLY_NOW: &'static str = "APPLY_NOW";
    pub const BOOK_NOW: &'static str = "BOOK_NOW";
    pub const BUY_TICKETS: &'static str = "BUY_TICKETS";
    pub const CONTACT_US: &'static str = "CONTACT_US";
    pub const DONATE: &'static str = "DONATE";
    pub const DOWNLOAD: &'static str = "DOWNLOAD";
    pub const GET_NOW: &'static str = "GET_NOW";
    pub const INSTALL_NOW: &'static str = "INSTALL_NOW";
    pub const LEARN_MORE: &'static str = "LEARN_MORE";
    pub const LISTEN: &'static str = "LISTEN";
    pub const MORE: &'static str = "MORE";
    pub const ORDER_NOW: &'static str = "ORDER_NOW";
    pub const PLAY: &'static str = "PLAY";
    pub const READ: &'static str = "READ";
    pub const SHOP_NOW: &'static str = "SHOP_NOW";
    pub const SHOW_TIMES: &'static str = "SHOW_TIMES";
    pub const SIGN_UP: &'static str = "SIGN_UP";
    pub const SUBSCRIBE: &'static str = "SUBSCRIBE";
    pub const USE_APP: &'static str = "USE_APP";
    pub const VIEW: &'static str = "VIEW";
    pub const VIEW_MORE: &'static str = "VIEW_MORE";
    pub const VOTE_NOW: &'static str = "VOTE_NOW";
    pub const WATCH: &'static str = "WATCH";
}
/// Audience source types
pub struct AudienceSourceTypes;
impl AudienceSourceTypes {
    pub const FIRST_PARTY: &'static str = "FIRST_PARTY";
    pub const ENGAGEMENT_SNAPCHAT: &'static str = "ENGAGEMENT_SNAPCHAT";
    pub const PIXEL: &'static str = "PIXEL";
    pub const MOBILE_APP: &'static str = "MOBILE_APP";
    pub const LOOKALIKE: &'static str = "LOOKALIKE";
}
