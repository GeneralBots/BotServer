use super::*;

impl InternalUrls {
    // No localhost defaults - services must be configured via Vault or env vars
    pub const BOTMODELS_VISION_QRCODE: &'static str = "/api/vision/qrcode";
    /// Canonical path registered by botmodels: `totext`, no hyphen.
    pub const BOTMODELS_SPEECH_TO_TEXT: &'static str = "/api/speech/totext";
    pub const BOTMODELS_VISION_DESCRIBE_VIDEO: &'static str = "/api/vision/describe-video";
    pub const BOTMODELS_VISION_DESCRIBE_VIDEO_UNDERSCORE: &'static str =
        "/api/vision/describe_video";
    pub const BOTMODELS_VISION_OCR: &'static str = "/api/vision/ocr";
    pub const BOTMODELS_VISION_BARCODE: &'static str = "/api/vision/barcode";
    pub const BOTMODELS_HEALTH: &'static str = "/api/health";
}
impl ApiUrls {
    pub fn with_params(url: &str, params: &[(&str, &str)]) -> String {
        let mut result = url.to_string();
        for (key, value) in params {
            result = result.replace(&format!(":{key}"), value);
        }
        result
    }

    pub fn with_query(url: &str, params: &[(&str, &str)]) -> String {
        if params.is_empty() {
            return url.to_string();
        }

        let query = params
            .iter()
            .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{url}?{query}")
    }
}
