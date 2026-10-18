use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = canvases)]
pub struct DbCanvas {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub width: i32,
    pub height: i32,
    pub background_color: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_public: bool,
    pub is_template: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org_id: Uuid,
    pub bot_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = canvas_elements)]
pub struct DbCanvasElement {
    pub id: Uuid,
    pub canvas_id: Uuid,
    pub element_type: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub z_index: i32,
    pub locked: bool,
    pub properties: serde_json::Value,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = canvas_collaborators)]
pub struct DbCanvasCollaborator {
    pub id: Uuid,
    pub canvas_id: Uuid,
    pub user_id: Uuid,
    pub permission: String,
    pub added_by: Option<Uuid>,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = canvas_versions)]
pub struct DbCanvasVersion {
    pub id: Uuid,
    pub canvas_id: Uuid,
    pub version_number: i32,
    pub name: Option<String>,
    pub elements_snapshot: serde_json::Value,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = canvas_comments)]
pub struct DbCanvasComment {
    pub id: Uuid,
    pub canvas_id: Uuid,
    pub element_id: Option<Uuid>,
    pub parent_comment_id: Option<Uuid>,
    pub author_id: Uuid,
    pub content: String,
    pub x_position: Option<f64>,
    pub y_position: Option<f64>,
    pub resolved: bool,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canvas {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub width: i32,
    pub height: i32,
    pub background_color: String,
    pub thumbnail_url: Option<String>,
    pub is_public: bool,
    pub is_template: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub elements: Vec<CanvasElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasElement {
    pub id: Uuid,
    pub canvas_id: Uuid,
    pub element_type: ElementType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub z_index: i32,
    pub locked: bool,
    pub properties: ElementProperties,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    FreehandPath,
    Text,
    Image,
    Sticky,
    Frame,
    Connector,
}

impl ElementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Line => "line",
            Self::Arrow => "arrow",
            Self::FreehandPath => "freehand_path",
            Self::Text => "text",
            Self::Image => "image",
            Self::Sticky => "sticky",
            Self::Frame => "frame",
            Self::Connector => "connector",
        }
    }

    pub fn from_label(s: &str) -> Self {
        match s {
            "rectangle" => Self::Rectangle,
            "ellipse" => Self::Ellipse,
            "line" => Self::Line,
            "arrow" => Self::Arrow,
            "freehand_path" => Self::FreehandPath,
            "text" => Self::Text,
            "image" => Self::Image,
            "sticky" => Self::Sticky,
            "frame" => Self::Frame,
            "connector" => Self::Connector,
            _ => Self::Rectangle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ElementProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_arrow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_arrow: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub element_count: i64,
    pub is_public: bool,
    pub is_template: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCanvasRequest {
    pub name: String,
    pub description: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub background_color: Option<String>,
    pub is_template: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCanvasRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub background_color: Option<String>,
    pub is_public: Option<bool>,
    pub is_template: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateElementRequest {
    pub element_type: ElementType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: Option<f64>,
    pub properties: Option<ElementProperties>,
    pub z_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateElementRequest {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: Option<f64>,
    pub properties: Option<ElementProperties>,
    pub z_index: Option<i32>,
    pub locked: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub scale: Option<f64>,
    pub background: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Png,
    Svg,
    Pdf,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    pub format: ExportFormat,
    pub url: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub element_id: Option<Uuid>,
    pub parent_comment_id: Option<Uuid>,
    pub x_position: Option<f64>,
    pub y_position: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct AddCollaboratorRequest {
    pub user_id: Uuid,
    pub permission: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub is_public: Option<bool>,
    pub is_template: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UiListQuery {
    pub search: Option<String>,
    pub is_template: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CollaborationSession {
    pub canvas_id: Uuid,
    pub user_id: Uuid,
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub selection: Vec<Uuid>,
    pub connected_at: DateTime<Utc>,
}
