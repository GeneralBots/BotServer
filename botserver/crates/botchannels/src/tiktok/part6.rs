use super::*;

impl TikTokVideo {
    /// Get the video URL
    pub fn url(&self) -> Option<&str> {
        self.share_url.as_deref()
    }

    /// Get video creation time as DateTime
    pub fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.create_time
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
    }
}
