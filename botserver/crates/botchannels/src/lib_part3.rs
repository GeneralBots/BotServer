use super::*;

impl PostResult {
    pub fn success(channel_type: ChannelType, post_id: String, url: Option<String>) -> Self {
        Self {
            success: true,
            channel_type,
            post_id: Some(post_id),
            url,
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn error(channel_type: ChannelType, error: impl Into<String>) -> Self {
        Self {
            success: false,
            channel_type,
            post_id: None,
            url: None,
            error: Some(error.into()),
            metadata: HashMap::new(),
        }
    }
}
