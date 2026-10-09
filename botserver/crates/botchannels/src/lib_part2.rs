use super::*;

impl PostContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            image_urls: vec![],
            video_url: None,
            link: None,
            hashtags: vec![],
            mentions: vec![],
            scheduled_at: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_image(mut self, url: impl Into<String>) -> Self {
        self.image_urls.push(url.into());
        self
    }

    pub fn with_images(mut self, urls: Vec<String>) -> Self {
        self.image_urls.extend(urls);
        self
    }

    pub fn with_video(mut self, url: impl Into<String>) -> Self {
        self.video_url = Some(url.into());
        self
    }

    pub fn with_link(mut self, url: impl Into<String>) -> Self {
        self.link = Some(url.into());
        self
    }

    pub fn with_hashtags(mut self, tags: Vec<String>) -> Self {
        self.hashtags.extend(tags);
        self
    }

    pub fn with_mentions(mut self, mentions: Vec<String>) -> Self {
        self.mentions.extend(mentions);
        self
    }

    pub fn scheduled(mut self, at: chrono::DateTime<chrono::Utc>) -> Self {
        self.scheduled_at = Some(at);
        self
    }
}
