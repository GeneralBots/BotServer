use super::*;

pub use botlib::db_pool::DbPool;
pub type GetDefaultBotFn = Arc<dyn Fn(&mut diesel::PgConnection) -> (Uuid, String) + Send + Sync>;
#[derive(Clone)]
pub struct SocialState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::social_communities;

pub use botschema::social::social_community_members;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::social_posts;

pub use botschema::social::social_comments;

pub use botschema::social::social_reactions;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::social_praises;

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = social_posts)]
pub struct DbPost {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub author_id: Uuid,
    pub community_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub content_type: String,
    pub attachments: serde_json::Value,
    pub mentions: serde_json::Value,
    pub hashtags: Vec<Option<String>>,
    pub visibility: String,
    pub is_announcement: bool,
    pub is_pinned: bool,
    pub poll_id: Option<Uuid>,
    pub reaction_counts: serde_json::Value,
    pub comment_count: i32,
    pub share_count: i32,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub branch_id: Option<Uuid>,
}
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = social_communities)]
pub struct DbCommunity {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub icon: Option<String>,
    pub visibility: String,
    pub join_policy: String,
    pub owner_id: Uuid,
    pub member_count: i32,
    pub post_count: i32,
    pub is_official: bool,
    pub is_featured: bool,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub branch_id: Option<Uuid>,
}
#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = social_community_members)]
pub struct DbCommunityMember {
    pub id: Uuid,
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub notifications_enabled: bool,
    pub joined_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = social_comments)]
pub struct DbComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub parent_comment_id: Option<Uuid>,
    pub author_id: Uuid,
    pub content: String,
    pub mentions: serde_json::Value,
    pub reaction_counts: serde_json::Value,
    pub reply_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = social_reactions)]
pub struct DbReaction {
    pub id: Uuid,
    pub post_id: Option<Uuid>,
    pub comment_id: Option<Uuid>,
    pub user_id: Uuid,
    pub reaction_type: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = social_praises)]
pub struct DbPraise {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub badge_type: String,
    pub message: Option<String>,
    pub is_public: bool,
    pub post_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub org_id: Uuid,
    pub bot_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub author_id: Uuid,
    pub community_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub content_type: String,
    pub attachments: Vec<Attachment>,
    pub mentions: Vec<Uuid>,
    pub hashtags: Vec<String>,
    pub visibility: String,
    pub is_announcement: bool,
    pub is_pinned: bool,
    pub poll_id: Option<Uuid>,
    pub reaction_counts: HashMap<String, i32>,
    pub comment_count: i32,
    pub share_count: i32,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Uuid,
    pub file_type: String,
    pub url: String,
    pub name: String,
    pub size: i64,
    pub mime_type: String,
    pub thumbnail_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub cover_image: Option<String>,
    pub icon: Option<String>,
    pub visibility: String,
    pub join_policy: String,
    pub owner_id: Uuid,
    pub member_count: i32,
    pub post_count: i32,
    pub is_official: bool,
    pub is_featured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityMember {
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub notifications_enabled: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub reaction_type: String,
    pub created_at: DateTime<Utc>,
}
