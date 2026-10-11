#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionEffect {
    Fade,
    Slide,
    Zoom,
    Wipe,
    None,
}

impl TransitionEffect {
    pub fn as_ooxml(&self) -> &'static str {
        match self {
            Self::Fade => "fade",
            Self::Slide => "slide",
            Self::Zoom => "zoom",
            Self::Wipe => "wipe",
            Self::None => "none",
        }
    }

    pub fn from_ooxml_name(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fade" => Self::Fade,
            "slide" | "slide-left" | "slide-right" => Self::Slide,
            "zoom" | "zoom-in" | "zoom-out" => Self::Zoom,
            "wipe" | "wipe-left" | "wipe-right" | "wipe-up" | "wipe-down" => Self::Wipe,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SlideElement {
    pub element_type: SlideElementType,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub rotation: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum SlideElementType {
    Text {
        content: String,
        font_size: Option<u32>,
        bold: bool,
        italic: bool,
        underline: bool,
        color: Option<String>,
        font_family: Option<String>,
        alignment: Option<String>,
    },
    Image {
        data: Vec<u8>,
        mime_type: String,
        alt_text: Option<String>,
    },
    Shape {
        shape_type: String,
        fill_color: Option<String>,
        line_color: Option<String>,
        text: Option<String>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

#[derive(Debug, Clone)]
pub struct Slide {
    pub title: Option<String>,
    pub body: Option<String>,
    pub elements: Vec<SlideElement>,
    pub background_color: Option<String>,
    pub background_image: Option<Vec<u8>>,
    pub notes: Option<String>,
    pub transition: TransitionEffect,
    pub transition_duration: u32,
    pub layout: SlideLayout,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlideLayout {
    Title,
    TitleAndContent,
    TwoContent,
    Blank,
    SectionHeader,
    Comparison,
    ContentWithCaption,
}

impl SlideLayout {
    pub fn as_ooxml_id(&self) -> u32 {
        match self {
            Self::Title => 0,
            Self::TitleAndContent => 1,
            Self::SectionHeader => 2,
            Self::TwoContent => 3,
            Self::Comparison => 4,
            Self::ContentWithCaption => 5,
            Self::Blank => 6,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PptxStyles {
    pub slide_width: u32,
    pub slide_height: u32,
    pub default_font: String,
    pub heading_font: String,
    pub title_color: String,
    pub body_color: String,
    pub accent_color: String,
    pub background_color: String,
}

impl Default for PptxStyles {
    fn default() -> Self {
        Self {
            slide_width: 12192000,
            slide_height: 6858000,
            default_font: "Calibri".to_string(),
            heading_font: "Calibri Light".to_string(),
            title_color: "1F3864".to_string(),
            body_color: "333333".to_string(),
            accent_color: "2E75B6".to_string(),
            background_color: "FFFFFF".to_string(),
        }
    }
}

pub struct PptxDocument {
    pub slides: Vec<Slide>,
    pub styles: PptxStyles,
    pub author: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
}

impl Default for PptxDocument {
    fn default() -> Self {
        Self::new()
    }
}

pub use super::pptx_html::from_html;
pub use super::pptx_json::from_json;
