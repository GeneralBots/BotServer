use super::*;

impl TikTokScopes {
    /// Basic user info (open_id, union_id, avatar)
    pub const USER_INFO_BASIC: &'static str = "user.info.basic";
    /// Extended user profile (bio, verified status, stats)
    pub const USER_INFO_PROFILE: &'static str = "user.info.profile";
    /// User's email address
    pub const USER_INFO_STATS: &'static str = "user.info.stats";
    /// List user's videos
    pub const VIDEO_LIST: &'static str = "video.list";
    /// Upload and publish videos
    pub const VIDEO_PUBLISH: &'static str = "video.publish";
    /// Upload videos to user's inbox
    pub const VIDEO_UPLOAD: &'static str = "video.upload";

    /// Get recommended scopes for content posting
    pub fn content_posting_scopes() -> &'static str {
        "user.info.basic,video.publish"
    }

    /// Get all available scopes
    pub fn all_scopes() -> &'static str {
        "user.info.basic,user.info.profile,user.info.stats,video.list,video.publish,video.upload"
    }
}

// ============================================================================
// Helper Functions
// ============================================================================
/// Build hashtag string from list
pub fn build_hashtags(tags: &[String]) -> String {
    tags.iter()
        .map(|t| {
            if t.starts_with('#') {
                t.clone()
            } else {
                format!("#{}", t)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
/// Validate video file for TikTok upload
pub fn validate_video_file(size_bytes: u64, duration_seconds: u32) -> Result<(), ChannelError> {
    // TikTok limits: 4GB max, typically 60s for regular users, up to 10 min for some
    const MAX_SIZE: u64 = 4 * 1024 * 1024 * 1024; // 4GB
    const MAX_DURATION: u32 = 600; // 10 minutes

    if size_bytes > MAX_SIZE {
        return Err(ChannelError::ApiError {
            code: Some("VIDEO_TOO_LARGE".to_string()),
            message: format!(
                "Video file too large: {} bytes (max: {} bytes)",
                size_bytes, MAX_SIZE
            ),
        });
    }

    if duration_seconds > MAX_DURATION {
        return Err(ChannelError::ApiError {
            code: Some("VIDEO_TOO_LONG".to_string()),
            message: format!(
                "Video too long: {} seconds (max: {} seconds)",
                duration_seconds, MAX_DURATION
            ),
        });
    }

    Ok(())
}
