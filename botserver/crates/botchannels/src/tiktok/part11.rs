use super::*;

impl ImageFormats {
    pub const JPEG: &'static str = "image/jpeg";
    pub const PNG: &'static str = "image/png";
    pub const WEBP: &'static str = "image/webp";

    pub fn is_supported(content_type: &str) -> bool {
        matches!(
            content_type.to_lowercase().as_str(),
            "image/jpeg" | "image/png" | "image/webp"
        )
    }
}
