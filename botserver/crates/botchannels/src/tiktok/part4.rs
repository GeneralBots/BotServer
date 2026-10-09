use super::*;

impl PublishStatus {
    pub fn is_complete(&self) -> bool {
        self.status == "PUBLISH_COMPLETE"
    }

    pub fn is_failed(&self) -> bool {
        self.status == "FAILED"
    }

    pub fn is_processing(&self) -> bool {
        matches!(
            self.status.as_str(),
            "PROCESSING_UPLOAD" | "PROCESSING_DOWNLOAD" | "SEND_TO_USER_INBOX"
        )
    }

    pub fn get_video_id(&self) -> Option<&str> {
        self.publicly_available_post_id
            .as_ref()
            .and_then(|ids| ids.first())
            .map(|s| s.as_str())
    }
}
