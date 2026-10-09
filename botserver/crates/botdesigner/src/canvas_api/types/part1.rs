use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canvas {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub width: f64,
    pub height: f64,
    pub background_color: String,
    pub grid_enabled: bool,
    pub grid_size: i32,
    pub snap_to_grid: bool,
    pub zoom_level: f64,
    pub elements: Vec<CanvasElement>,
    pub layers: Vec<Layer>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasElement {
    pub id: Uuid,
    pub element_type: ElementType,
    pub layer_id: Uuid,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub opacity: f64,
    pub visible: bool,
    pub locked: bool,
    pub name: Option<String>,
    pub style: ElementStyle,
    pub properties: ElementProperties,
    pub z_index: i32,
    pub parent_id: Option<Uuid>,
    pub children: Vec<Uuid>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Polygon,
    Path,
    Text,
    Image,
    Icon,
    Group,
    Frame,
    Component,
    Html,
    Svg,
}
impl std::fmt::Display for ElementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rectangle => write!(f, "rectangle"),
            Self::Ellipse => write!(f, "ellipse"),
            Self::Line => write!(f, "line"),
            Self::Arrow => write!(f, "arrow"),
            Self::Polygon => write!(f, "polygon"),
            Self::Path => write!(f, "path"),
            Self::Text => write!(f, "text"),
            Self::Image => write!(f, "image"),
            Self::Icon => write!(f, "icon"),
            Self::Group => write!(f, "group"),
            Self::Frame => write!(f, "frame"),
            Self::Component => write!(f, "component"),
            Self::Html => write!(f, "html"),
            Self::Svg => write!(f, "svg"),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementStyle {
    pub fill: Option<FillStyle>,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<ShadowStyle>,
    pub blur: Option<f64>,
    pub border_radius: Option<BorderRadius>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillStyle {
    pub fill_type: FillType,
    pub color: Option<String>,
    pub gradient: Option<Gradient>,
    pub pattern: Option<PatternFill>,
    pub opacity: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillType {
    Solid,
    LinearGradient,
    RadialGradient,
    Pattern,
    None,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gradient {
    pub stops: Vec<GradientStop>,
    pub angle: f64,
    pub center_x: Option<f64>,
    pub center_y: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f64,
    pub color: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternFill {
    pub pattern_type: String,
    pub scale: f64,
    pub rotation: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub color: String,
    pub width: f64,
    pub dash_array: Option<Vec<f64>>,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub opacity: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineCap {
    Butt,
    Round,
    Square,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowStyle {
    pub color: String,
    pub blur: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub spread: f64,
    pub inset: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderRadius {
    pub top_left: f64,
    pub top_right: f64,
    pub bottom_right: f64,
    pub bottom_left: f64,
}
impl BorderRadius {
    pub fn uniform(radius: f64) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementProperties {
    pub text_content: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<String>,
    pub font_style: Option<String>,
    pub text_align: Option<TextAlign>,
    pub vertical_align: Option<VerticalAlign>,
    pub line_height: Option<f64>,
    pub letter_spacing: Option<f64>,
    pub text_decoration: Option<String>,
    pub text_color: Option<String>,
    pub image_url: Option<String>,
    pub image_fit: Option<ImageFit>,
    pub icon_name: Option<String>,
    pub icon_set: Option<String>,
    pub html_content: Option<String>,
    pub svg_content: Option<String>,
    pub path_data: Option<String>,
    pub points: Option<Vec<Point>>,
    pub arrow_start: Option<ArrowHead>,
    pub arrow_end: Option<ArrowHead>,
    pub component_id: Option<Uuid>,
    pub component_props: Option<HashMap<String, serde_json::Value>>,
    pub constraints: Option<Constraints>,
    pub auto_layout: Option<AutoLayout>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrowHead {
    None,
    Triangle,
    Circle,
    Diamond,
    Square,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub horizontal: ConstraintType,
    pub vertical: ConstraintType,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    Fixed,
    Min,
    Max,
    Center,
    Scale,
    Stretch,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoLayout {
    pub direction: LayoutDirection,
    pub spacing: f64,
    pub padding_top: f64,
    pub padding_right: f64,
    pub padding_bottom: f64,
    pub padding_left: f64,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
    pub wrap: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignItems {
    Start,
    Center,
    End,
    Stretch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f64,
    pub blend_mode: BlendMode,
    pub z_index: i32,
}
