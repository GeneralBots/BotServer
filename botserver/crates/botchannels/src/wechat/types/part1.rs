use super::*;

#[derive(Debug, Clone)]
pub(crate) struct CachedToken {
    pub(crate) access_token: String,
    pub(crate) expires_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: Option<String>,
    pub expires_in: Option<u64>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}

// ============================================================================
// API Response Types
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatApiResponse<T> {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    #[serde(flatten)]
    pub data: Option<T>,
    pub msgid: Option<i64>,
}

// ============================================================================
// Message Types
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMessage {
    pub touser: String,
    pub template_id: String,
    pub url: Option<String>,
    pub miniprogram: Option<MiniProgram>,
    pub data: HashMap<String, TemplateDataItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniProgram {
    pub appid: String,
    pub pagepath: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDataItem {
    pub value: String,
    pub color: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMessageResult {
    pub msgid: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "msgtype", rename_all = "lowercase")]
pub enum CustomerMessage {
    Text {
        touser: String,
        text: TextContent,
    },
    Image {
        touser: String,
        image: MediaContent,
    },
    Voice {
        touser: String,
        voice: MediaContent,
    },
    Video {
        touser: String,
        video: VideoContent,
    },
    Music {
        touser: String,
        music: MusicContent,
    },
    News {
        touser: String,
        news: NewsContent,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaContent {
    pub media_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoContent {
    pub media_id: String,
    pub thumb_media_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicContent {
    pub title: Option<String>,
    pub description: Option<String>,
    pub musicurl: String,
    pub hqmusicurl: String,
    pub thumb_media_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsContent {
    pub articles: Vec<NewsItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub picurl: Option<String>,
}

// ============================================================================
// Media Types
// ============================================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Image,
    Voice,
    Video,
    Thumb,
}
impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Voice => "voice",
            Self::Video => "video",
            Self::Thumb => "thumb",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Image => "image/jpeg",
            Self::Voice => "audio/amr",
            Self::Video => "video/mp4",
            Self::Thumb => "image/jpeg",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaUploadResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    pub media_id: Option<String>,
    pub created_at: Option<i64>,
}
#[derive(Debug, Clone)]
pub struct MediaUploadResult {
    pub media_type: String,
    pub media_id: String,
    pub created_at: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDescription {
    pub title: String,
    pub introduction: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermanentMediaResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub media_id: Option<String>,
    pub url: Option<String>,
}
#[derive(Debug, Clone)]
pub struct PermanentMediaResult {
    pub media_id: String,
    pub url: Option<String>,
}

// ============================================================================
// Content Publishing Types
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    pub title: String,
    pub author: Option<String>,
    pub digest: Option<String>,
    pub content: String,
    pub content_source_url: Option<String>,
    pub thumb_media_id: String,
    pub need_open_comment: Option<i32>,
    pub only_fans_can_comment: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub media_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub publish_id: Option<String>,
}
#[derive(Debug, Clone)]
pub struct PublishResult {
    pub publish_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishStatusResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub publish_status: Option<i32>,
    pub article_id: Option<String>,
    pub article_detail: Option<ArticleDetail>,
    pub fail_idx: Option<Vec<i32>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleDetail {
    pub count: Option<i32>,
    pub item: Option<Vec<ArticleItem>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleItem {
    pub idx: Option<i32>,
    pub article_url: Option<String>,
}
#[derive(Debug, Clone)]
pub struct PublishStatus {
    pub publish_id: String,
    pub publish_status: i32, // 0=success, 1=publishing, 2=failed
    pub article_id: Option<String>,
    pub article_detail: Option<ArticleDetail>,
    pub fail_idx: Option<Vec<i32>>,
}
impl PublishStatus {
    pub fn is_success(&self) -> bool {
        self.publish_status == 0
    }

    pub fn is_publishing(&self) -> bool {
        self.publish_status == 1
    }

    pub fn is_failed(&self) -> bool {
        self.publish_status == 2
    }
}

// ============================================================================
// User Types
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatUserResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub subscribe: Option<i32>,
    pub openid: Option<String>,
    pub nickname: Option<String>,
    pub sex: Option<i32>,
    pub language: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub country: Option<String>,
    pub headimgurl: Option<String>,
    pub subscribe_time: Option<i64>,
    pub unionid: Option<String>,
    pub remark: Option<String>,
    pub groupid: Option<i32>,
    pub tagid_list: Option<Vec<i32>>,
    pub subscribe_scene: Option<String>,
    pub qr_scene: Option<i32>,
    pub qr_scene_str: Option<String>,
}
#[derive(Debug, Clone)]
pub struct WeChatUser {
    pub subscribe: i32,
    pub openid: String,
    pub nickname: Option<String>,
    pub sex: Option<i32>,
    pub language: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub country: Option<String>,
    pub headimgurl: Option<String>,
    pub subscribe_time: Option<i64>,
    pub unionid: Option<String>,
    pub remark: Option<String>,
    pub groupid: Option<i32>,
    pub tagid_list: Option<Vec<i32>>,
    pub subscribe_scene: Option<String>,
    pub qr_scene: Option<i32>,
    pub qr_scene_str: Option<String>,
}
impl WeChatUser {
    pub fn is_subscribed(&self) -> bool {
        self.subscribe == 1
    }
}
