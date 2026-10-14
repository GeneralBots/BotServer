use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<WorkspaceIcon>,
    pub cover_image: Option<String>,
    pub settings: WorkspaceSettings,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub members: Vec<WorkspaceMember>,
    pub root_pages: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceIcon {
    pub icon_type: IconType,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IconType {
    Emoji,
    Image,
    Lucide,
}

impl IconType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Emoji => "emoji",
            Self::Image => "image",
            Self::Lucide => "lucide",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "emoji" => Self::Emoji,
            "image" => Self::Image,
            "lucide" => Self::Lucide,
            _ => Self::Emoji,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMember {
    pub user_id: Uuid,
    pub role: WorkspaceRole,
    pub joined_at: DateTime<Utc>,
    pub invited_by: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Editor,
    Commenter,
    Viewer,
}

impl WorkspaceRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Editor => "editor",
            Self::Commenter => "commenter",
            Self::Viewer => "viewer",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "owner" => Self::Owner,
            "admin" => Self::Admin,
            "editor" => Self::Editor,
            "commenter" => Self::Commenter,
            "viewer" => Self::Viewer,
            _ => Self::Viewer,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceSettings {
    #[serde(default)]
    pub default_page_width: PageWidth,
    #[serde(default)]
    pub allow_public_pages: bool,
    #[serde(default = "default_true")]
    pub enable_comments: bool,
    #[serde(default = "default_true")]
    pub enable_reactions: bool,
    #[serde(default = "default_true")]
    pub enable_gb_assist: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gb_bot_id: Option<Uuid>,
}

pub(crate) fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PageWidth {
    Small,
    #[default]
    Normal,
    Wide,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
    pub cover_image: Option<String>,
    pub blocks: Vec<Block>,
    pub children: Vec<Uuid>,
    pub properties: HashMap<String, PropertyValue>,
    pub permissions: PagePermissions,
    pub is_template: bool,
    pub template_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub last_edited_by: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PagePermissions {
    #[serde(default = "default_true")]
    pub inherit_from_parent: bool,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub public_edit: bool,
    #[serde(default)]
    pub allowed_users: Vec<Uuid>,
    #[serde(default)]
    pub allowed_roles: Vec<WorkspaceRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub block_type: BlockType,
    pub content: BlockContent,
    pub properties: BlockProperties,
    pub children: Vec<Block>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    BulletedList,
    NumberedList,
    Checklist,
    Toggle,
    Quote,
    Callout,
    Divider,
    Table,
    Code,
    Image,
    Video,
    File,
    Embed,
    Bookmark,
    LinkToPage,
    SyncedBlock,
    TableOfContents,
    Breadcrumb,
    Equation,
    ColumnList,
    Column,
    GbComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Default)]
pub enum BlockContent {
    Text { text: RichText },
    Media { url: String, caption: Option<String> },
    Table { rows: Vec<TableRow> },
    Code { code: String, language: Option<String> },
    Embed { url: String, embed_type: Option<String> },
    Callout { icon: Option<String>, text: RichText },
    Toggle { title: RichText, expanded: bool },
    Checklist { items: Vec<ChecklistItem> },
    GbComponent { component_type: String, config: serde_json::Value },
    #[default]
    Empty,
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RichText {
    pub segments: Vec<TextSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSegment {
    pub text: String,
    #[serde(default)]
    pub annotations: TextAnnotations,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention: Option<Mention>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextAnnotations {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub strikethrough: bool,
    #[serde(default)]
    pub code: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    pub mention_type: MentionType,
    pub target_id: Uuid,
    pub display_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MentionType {
    User,
    Page,
    Date,
    Database,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub id: Uuid,
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub content: RichText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: Uuid,
    pub text: RichText,
    #[serde(default)]
    pub checked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(default)]
    pub indent_level: u32,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropertyValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    Date(DateTime<Utc>),
    Select(String),
    MultiSelect(Vec<String>),
    User(Uuid),
    Users(Vec<Uuid>),
    Url(String),
    Email(String),
    Phone(String),
    Relation(Vec<Uuid>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageVersion {
    pub id: Uuid,
    pub page_id: Uuid,
    pub version_number: i32,
    pub title: String,
    pub blocks: Vec<Block>,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTreeNode {
    pub id: Uuid,
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
    pub children: Vec<PageTreeNode>,
    pub has_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSearchResult {
    pub page_id: Uuid,
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
    pub snippet: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: SlashCommandCategory,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SlashCommandCategory {
    GbAssist,
    General,
    Media,
    Embed,
    Advanced,
}

#[derive(Debug, Clone)]
pub enum WorkspacesError {
    WorkspaceNotFound,
    PageNotFound,
    BlockNotFound,
    CommentNotFound,
    VersionNotFound,
    MemberNotFound,
    MemberAlreadyExists,
    CannotRemoveLastOwner,
    PermissionDenied,
    InvalidOperation(String),
    DbError(String),
}

impl std::fmt::Display for WorkspacesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceNotFound => write!(f, "Workspace not found"),
            Self::PageNotFound => write!(f, "Page not found"),
            Self::BlockNotFound => write!(f, "Block not found"),
            Self::CommentNotFound => write!(f, "Comment not found"),
            Self::VersionNotFound => write!(f, "Version not found"),
            Self::MemberNotFound => write!(f, "Member not found"),
            Self::MemberAlreadyExists => write!(f, "Member already exists in workspace"),
            Self::CannotRemoveLastOwner => write!(f, "Cannot remove the last owner"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::InvalidOperation(e) => write!(f, "Invalid operation: {e}"),
            Self::DbError(e) => write!(f, "Database error: {e}"),
        }
    }
}

impl std::error::Error for WorkspacesError {}

impl IntoResponse for WorkspacesError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            Self::WorkspaceNotFound
            | Self::PageNotFound
            | Self::BlockNotFound
            | Self::CommentNotFound
            | Self::VersionNotFound
            | Self::MemberNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::PermissionDenied => (StatusCode::FORBIDDEN, self.to_string()),
            Self::MemberAlreadyExists | Self::CannotRemoveLastOwner | Self::InvalidOperation(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            Self::DbError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()),
        };
        (status, Json(serde_json::json!({"error": message}))).into_response()
    }
}

