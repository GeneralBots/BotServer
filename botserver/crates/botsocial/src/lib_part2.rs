use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub parent_comment_id: Option<Uuid>,
    pub author_id: Uuid,
    pub content: String,
    pub mentions: Vec<Uuid>,
    pub reaction_counts: HashMap<String, i32>,
    pub reply_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poll {
    pub id: Uuid,
    pub post_id: Uuid,
    pub question: String,
    pub options: Vec<PollOption>,
    pub allow_multiple: bool,
    pub allow_add_options: bool,
    pub anonymous: bool,
    pub ends_at: Option<DateTime<Utc>>,
    pub total_votes: i32,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollOption {
    pub id: Uuid,
    pub text: String,
    pub vote_count: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Praise {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub badge_type: String,
    pub message: String,
    pub is_public: bool,
    pub post_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    pub community_id: Option<Uuid>,
    pub author_id: Option<Uuid>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub content: String,
    pub content_type: Option<String>,
    pub community_id: Option<Uuid>,
    pub visibility: Option<String>,
    pub mentions: Option<Vec<Uuid>>,
    pub hashtags: Option<Vec<String>>,
}
#[derive(Debug, Deserialize)]
pub struct CreatePostForm {
    pub content: String,
    pub visibility: Option<String>,
    pub community_id: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub content: Option<String>,
    pub visibility: Option<String>,
    pub is_pinned: Option<bool>,
}
#[derive(Debug, Deserialize)]
pub struct CreateCommunityRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<String>,
    pub join_policy: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct UpdateCommunityRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub icon: Option<String>,
    pub visibility: Option<String>,
    pub join_policy: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub parent_comment_id: Option<Uuid>,
    pub mentions: Option<Vec<Uuid>>,
}
#[derive(Debug, Deserialize)]
pub struct CreateReactionRequest {
    pub reaction_type: String,
}
#[derive(Debug, Deserialize)]
pub struct CreatePollRequest {
    pub question: String,
    pub options: Vec<String>,
    pub allow_multiple: Option<bool>,
    pub allow_add_options: Option<bool>,
    pub anonymous: Option<bool>,
    pub ends_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Deserialize)]
pub struct VotePollRequest {
    pub option_ids: Vec<Uuid>,
}
#[derive(Debug, Deserialize)]
pub struct CreatePraiseRequest {
    pub to_user_id: Uuid,
    pub badge_type: String,
    pub message: String,
    pub is_public: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct FeedResponse {
    pub posts: Vec<PostWithAuthor>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct PostWithAuthor {
    #[serde(flatten)]
    pub post: Post,
    pub author: UserSummary,
    pub community: Option<CommunitySummary>,
    pub user_reaction: Option<String>,
    pub is_bookmarked: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
    pub title: Option<String>,
    pub is_leader: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct CommunitySummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub icon: Option<String>,
}
pub(super) fn db_post_to_post(db: DbPost) -> Post {
    let attachments: Vec<Attachment> = serde_json::from_value(db.attachments).unwrap_or_default();
    let mentions: Vec<Uuid> = serde_json::from_value(db.mentions).unwrap_or_default();
    let reaction_counts: HashMap<String, i32> =
        serde_json::from_value(db.reaction_counts).unwrap_or_default();
    let hashtags: Vec<String> = db.hashtags.into_iter().flatten().collect();

    Post {
        id: db.id,
        organization_id: db.org_id,
        author_id: db.author_id,
        community_id: db.community_id,
        parent_id: db.parent_id,
        content: db.content,
        content_type: db.content_type,
        attachments,
        mentions,
        hashtags,
        visibility: db.visibility,
        is_announcement: db.is_announcement,
        is_pinned: db.is_pinned,
        poll_id: db.poll_id,
        reaction_counts,
        comment_count: db.comment_count,
        share_count: db.share_count,
        view_count: db.view_count,
        created_at: db.created_at,
        updated_at: db.updated_at,
        edited_at: db.edited_at,
        deleted_at: db.deleted_at,
    }
}
pub(super) fn db_community_to_community(db: DbCommunity) -> Community {
    Community {
        id: db.id,
        organization_id: db.org_id,
        name: db.name,
        slug: db.slug,
        description: db.description.unwrap_or_default(),
        cover_image: db.cover_image,
        icon: db.icon,
        visibility: db.visibility,
        join_policy: db.join_policy,
        owner_id: db.owner_id,
        member_count: db.member_count,
        post_count: db.post_count,
        is_official: db.is_official,
        is_featured: db.is_featured,
        created_at: db.created_at,
        updated_at: db.updated_at,
        archived_at: db.archived_at,
    }
}
pub(super) fn db_comment_to_comment(db: DbComment) -> Comment {
    let mentions: Vec<Uuid> = serde_json::from_value(db.mentions).unwrap_or_default();
    let reaction_counts: HashMap<String, i32> =
        serde_json::from_value(db.reaction_counts).unwrap_or_default();

    Comment {
        id: db.id,
        post_id: db.post_id,
        parent_comment_id: db.parent_comment_id,
        author_id: db.author_id,
        content: db.content,
        mentions,
        reaction_counts,
        reply_count: db.reply_count,
        created_at: db.created_at,
        updated_at: db.updated_at,
        edited_at: db.edited_at,
        deleted_at: db.deleted_at,
    }
}
pub(super) fn db_member_to_member(db: DbCommunityMember) -> CommunityMember {
    CommunityMember {
        community_id: db.community_id,
        user_id: db.user_id,
        role: db.role,
        joined_at: db.joined_at,
        notifications_enabled: db.notifications_enabled,
        last_seen_at: db.last_seen_at,
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SocialError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Internal error: {0}")]
    Internal(String),
}
impl IntoResponse for SocialError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let (status, message) = match &self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            Self::Database(msg) | Self::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
