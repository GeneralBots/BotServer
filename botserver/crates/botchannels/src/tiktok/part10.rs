use super::*;

impl VideoFormats {
    pub const MP4: &'static str = "video/mp4";
    pub const WEBM: &'static str = "video/webm";
    pub const MOV: &'static str = "video/quicktime";

    pub fn is_supported(content_type: &str) -> bool {
        matches!(
            content_type.to_lowercase().as_str(),
            "video/mp4" | "video/webm" | "video/quicktime"
        )
    }
}
