use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub thumbnail_url: Option<String>,
    pub canvas_data: serde_json::Value,
    pub is_system: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLibraryItem {
    pub id: Uuid,
    pub name: String,
    pub asset_type: AssetType,
    pub url: Option<String>,
    pub svg_content: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub is_system: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Icon,
    Image,
    Illustration,
    Shape,
    Component,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCanvasRequest {
    pub name: String,
    pub description: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub template_id: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCanvasRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub background_color: Option<String>,
    pub grid_enabled: Option<bool>,
    pub grid_size: Option<i32>,
    pub snap_to_grid: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddElementRequest {
    pub element_type: ElementType,
    pub layer_id: Option<Uuid>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub style: Option<ElementStyle>,
    pub properties: Option<ElementProperties>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateElementRequest {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: Option<f64>,
    pub scale_x: Option<f64>,
    pub scale_y: Option<f64>,
    pub opacity: Option<f64>,
    pub visible: Option<bool>,
    pub locked: Option<bool>,
    pub name: Option<String>,
    pub style: Option<ElementStyle>,
    pub properties: Option<ElementProperties>,
    pub z_index: Option<i32>,
    pub layer_id: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveElementRequest {
    pub delta_x: f64,
    pub delta_y: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeElementRequest {
    pub width: f64,
    pub height: f64,
    pub anchor: ResizeAnchor,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResizeAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupElementsRequest {
    pub element_ids: Vec<Uuid>,
    pub name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignElementsRequest {
    pub element_ids: Vec<Uuid>,
    pub alignment: Alignment,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    Left,
    CenterHorizontal,
    Right,
    Top,
    CenterVertical,
    Bottom,
    DistributeHorizontal,
    DistributeVertical,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLayerRequest {
    pub name: String,
    pub z_index: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLayerRequest {
    pub name: Option<String>,
    pub visible: Option<bool>,
    pub locked: Option<bool>,
    pub opacity: Option<f64>,
    pub blend_mode: Option<BlendMode>,
    pub z_index: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub quality: Option<i32>,
    pub scale: Option<f64>,
    pub background: Option<bool>,
    pub element_ids: Option<Vec<Uuid>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Png,
    Jpg,
    Svg,
    Pdf,
    Html,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub format: ExportFormat,
    pub data: String,
    pub content_type: String,
    pub filename: String,
    pub width: f64,
    pub height: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDesignRequest {
    pub prompt: String,
    pub context: Option<AiDesignContext>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDesignContext {
    pub selected_elements: Option<Vec<Uuid>>,
    pub canvas_state: Option<serde_json::Value>,
    pub style_preferences: Option<StylePreferences>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylePreferences {
    pub color_palette: Option<Vec<String>>,
    pub font_families: Option<Vec<String>>,
    pub design_style: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDesignResponse {
    pub success: bool,
    pub elements_created: Vec<CanvasElement>,
    pub elements_modified: Vec<Uuid>,
    pub message: String,
    pub html_preview: Option<String>,
    pub svg_preview: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasEvent {
    pub event_type: CanvasEventType,
    pub canvas_id: Uuid,
    pub user_id: Uuid,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasEventType {
    ElementAdded,
    ElementUpdated,
    ElementDeleted,
    ElementMoved,
    ElementResized,
    ElementsGrouped,
    ElementsUngrouped,
    LayerAdded,
    LayerUpdated,
    LayerDeleted,
    CanvasUpdated,
    SelectionChanged,
    CursorMoved,
    UndoPerformed,
    RedoPerformed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoRedoState {
    pub canvas_id: Uuid,
    pub undo_stack: Vec<CanvasSnapshot>,
    pub redo_stack: Vec<CanvasSnapshot>,
    pub max_history: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSnapshot {
    pub id: Uuid,
    pub elements: Vec<CanvasElement>,
    pub layers: Vec<Layer>,
    pub timestamp: DateTime<Utc>,
    pub description: String,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(super) fn test_element_type_display() {
        assert_eq!(ElementType::Rectangle.to_string(), "rectangle");
        assert_eq!(ElementType::Ellipse.to_string(), "ellipse");
        assert_eq!(ElementType::Text.to_string(), "text");
    }

    #[test]
    pub(super) fn test_border_radius_uniform() {
        let radius = BorderRadius::uniform(10.0);
        assert_eq!(radius.top_left, 10.0);
        assert_eq!(radius.top_right, 10.0);
        assert_eq!(radius.bottom_right, 10.0);
        assert_eq!(radius.bottom_left, 10.0);
    }

    #[test]
    pub(super) fn test_blend_mode_default() {
        let mode = BlendMode::default();
        assert_eq!(mode, BlendMode::Normal);
    }

    #[test]
    pub(super) fn test_element_style_default() {
        let style = ElementStyle::default();
        assert!(style.fill.is_none());
        assert!(style.stroke.is_none());
        assert!(style.blur.is_none());
    }
}
