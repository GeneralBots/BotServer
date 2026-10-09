use super::*;

impl TikTokUser {
    /// Get the user's TikTok profile URL
    pub fn profile_url(&self) -> Option<String> {
        self.profile_deep_link.clone()
    }
}
