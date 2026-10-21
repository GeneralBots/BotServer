use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = aiworkspaces)]
pub struct DbWorkspace {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    pub cover_image: Option<String>,
    pub settings: serde_json::Value,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = aiworkspace_members)]
pub struct DbWorkspaceMember {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub invited_by: Option<Uuid>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = aiworkspace_pages)]
pub struct DbWorkspacePage {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    pub cover_image: Option<String>,
    pub content: serde_json::Value,
    pub properties: serde_json::Value,
    pub is_template: bool,
    pub template_id: Option<Uuid>,
    pub is_public: bool,
    pub public_edit: bool,
    pub position: i32,
    pub created_by: Uuid,
    pub last_edited_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = aiworkspace_page_versions)]
pub struct DbPageVersion {
    pub id: Uuid,
    pub page_id: Uuid,
    pub version_number: i32,
    pub title: String,
    pub content: serde_json::Value,
    pub change_summary: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = aiworkspace_comments)]
pub struct DbWorkspaceComment {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub page_id: Uuid,
    pub block_id: Option<Uuid>,
    pub parent_comment_id: Option<Uuid>,
    pub author_id: Uuid,
    pub content: String,
    pub resolved: bool,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

