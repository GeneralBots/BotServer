use super::*;

pub type DbPool = diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;
pub type GetDefaultBotFn = Arc<dyn Fn(&mut diesel::PgConnection) -> (Uuid, String) + Send + Sync>;
#[derive(Clone)]
pub struct SocialState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
}

diesel::table! {
    social_communities (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        cover_image -> Nullable<Text>,
        icon -> Nullable<Text>,
        visibility -> Varchar,
        join_policy -> Varchar,
        owner_id -> Uuid,
        member_count -> Int4,
        post_count -> Int4,
        is_official -> Bool,
        is_featured -> Bool,
        settings -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        archived_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    social_community_members (id) {
        id -> Uuid,
        community_id -> Uuid,
        user_id -> Uuid,
        role -> Varchar,
        notifications_enabled -> Bool,
        joined_at -> Timestamptz,
        last_seen_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    social_posts (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        author_id -> Uuid,
        community_id -> Nullable<Uuid>,
        parent_id -> Nullable<Uuid>,
        content -> Text,
        content_type -> Varchar,
        attachments -> Jsonb,
        mentions -> Jsonb,
        hashtags -> Array<Nullable<Text>>,
        visibility -> Varchar,
        is_announcement -> Bool,
        is_pinned -> Bool,
        poll_id -> Nullable<Uuid>,
        reaction_counts -> Jsonb,
        comment_count -> Int4,
        share_count -> Int4,
        view_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        edited_at -> Nullable<Timestamptz>,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    social_comments (id) {
        id -> Uuid,
        post_id -> Uuid,
        parent_comment_id -> Nullable<Uuid>,
        author_id -> Uuid,
        content -> Text,
        mentions -> Jsonb,
        reaction_counts -> Jsonb,
        reply_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        edited_at -> Nullable<Timestamptz>,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    social_reactions (id) {
        id -> Uuid,
        post_id -> Nullable<Uuid>,
        comment_id -> Nullable<Uuid>,
        user_id -> Uuid,
        reaction_type -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    social_praises (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        from_user_id -> Uuid,
        to_user_id -> Uuid,
        badge_type -> Varchar,
        message -> Nullable<Text>,
        is_public -> Bool,
        post_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    social_communities,
    social_community_members,
    social_posts,
    social_comments,
    social_reactions,
    social_praises,
);
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
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub badge_type: String,
    pub message: Option<String>,
    pub is_public: bool,
    pub post_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
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
