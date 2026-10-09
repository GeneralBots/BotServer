use super::*;

impl CreatorInfo {
    /// Check if public posting is available
    pub fn can_post_public(&self) -> bool {
        self.privacy_level_options
            .as_ref()
            .map(|opts| opts.contains(&"PUBLIC_TO_EVERYONE".to_string()))
            .unwrap_or(false)
    }

    /// Get max video duration in seconds
    pub fn max_video_duration(&self) -> u32 {
        self.max_video_post_duration_sec.unwrap_or(60)
    }
}
