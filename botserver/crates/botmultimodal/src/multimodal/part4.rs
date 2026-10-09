use super::*;

impl BotModelsClient {
    pub fn new(
        config: BotModelsConfig,
        image_config: ImageGeneratorConfig,
        video_config: VideoGeneratorConfig,
    ) -> Self {
        let client = create_tls_client(Some(300));

        Self {
            client,
            config,
            image_config,
            video_config,
        }
    }

    pub fn from_provider_all(provider: &dyn ConfigProvider, bot_id: &Uuid) -> Self {
        let config = BotModelsConfig::from_provider(provider, bot_id);
        let image_config = ImageGeneratorConfig::from_provider(provider, bot_id);
        let video_config = VideoGeneratorConfig::from_provider(provider, bot_id);
        Self::new(config, image_config, video_config)
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn generate_image(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("BotModels is not enabled".into());
        }

        let url = format!("{}/api/image/generate", self.config.base_url());
        trace!("Generating image at {}: {}", url, prompt);

        let request = ImageGenerateRequest {
            prompt: prompt.to_string(),
            steps: Some(self.image_config.steps),
            width: Some(self.image_config.width),
            height: Some(self.image_config.height),
            guidance_scale: Some(7.5),
            seed: None,
        };

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Image generation failed: {}", error_text);
            return Err(format!("Image generation failed: {}", error_text).into());
        }

        let result: GenerationResponse = response.json().await?;

        if result.status == "completed" {
            if let Some(file_path) = result.file_path {
                let full_url = format!("{}{}", self.config.base_url(), file_path);
                info!("Image generated: {}", full_url);
                return Ok(full_url);
            }
        }

        Err(result
            .error
            .unwrap_or("Unknown error".to_string())
            .into())
    }

    pub async fn generate_video(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("BotModels is not enabled".into());
        }

        let url = format!("{}/api/video/generate", self.config.base_url());
        trace!("Generating video at {}: {}", url, prompt);

        let request = VideoGenerateRequest {
            prompt: prompt.to_string(),
            num_frames: Some(self.video_config.frames),
            fps: Some(self.video_config.fps),
            steps: Some(50),
            seed: None,
        };

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Video generation failed: {}", error_text);
            return Err(format!("Video generation failed: {}", error_text).into());
        }

        let result: GenerationResponse = response.json().await?;

        if result.status == "completed" {
            if let Some(file_path) = result.file_path {
                let full_url = format!("{}{}", self.config.base_url(), file_path);
                info!("Video generated: {}", full_url);
                return Ok(full_url);
            }
        }

        Err(result
            .error
            .unwrap_or("Unknown error".to_string())
            .into())
    }

    pub async fn generate_audio(
        &self,
        text: &str,
        voice: Option<&str>,
        language: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("BotModels is not enabled".into());
        }

        let url = format!("{}/api/speech/generate", self.config.base_url());
        trace!("Generating audio at {}: {}", url, text);

        let request = SpeechGenerateRequest {
            prompt: text.to_string(),
            voice: voice.map(String::from),
            language: language.map(String::from),
        };

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Audio generation failed: {}", error_text);
            return Err(format!("Audio generation failed: {}", error_text).into());
        }

        let result: GenerationResponse = response.json().await?;

        if result.status == "completed" {
            if let Some(file_path) = result.file_path {
                let full_url = format!("{}{}", self.config.base_url(), file_path);
                info!("Audio generated: {}", full_url);
                return Ok(full_url);
            }
        }

        Err(result
            .error
            .unwrap_or("Unknown error".to_string())
            .into())
    }

    pub async fn describe_image(
        &self,
        image_url_or_path: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("BotModels is not enabled".into());
        }

        let url = format!("{}/api/vision/describe", self.config.base_url());
        trace!("Describing image at {}: {}", url, image_url_or_path);

        let image_data = if image_url_or_path.starts_with("http") {
            let response = self.client.get(image_url_or_path).send().await?;
            response.bytes().await?.to_vec()
        } else {
            tokio::fs::read(image_url_or_path).await?
        };

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(image_data)
                .file_name("image.png")
                .mime_str("image/png")?,
        );

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Image description failed: {}", error_text);
            return Err(format!("Image description failed: {}", error_text).into());
        }

        let result: DescribeResponse = response.json().await?;
        info!("Image described: {}", result.description);
        Ok(result.description)
    }

    pub async fn describe_video(
        &self,
        video_url_or_path: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("BotModels is not enabled".into());
        }

        // botmodels registers the hyphenated route; the underscore spelling only
        // exists as a compatibility alias (issue 1514).
        let url = format!(
            "{}{}",
            self.config.base_url(),
            InternalUrls::BOTMODELS_VISION_DESCRIBE_VIDEO
        );
        trace!("Describing video at {}: {}", url, video_url_or_path);

        let video_data = if video_url_or_path.starts_with("http") {
            let response = self.client.get(video_url_or_path).send().await?;
            response.bytes().await?.to_vec()
        } else {
            tokio::fs::read(video_url_or_path).await?
        };

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(video_data)
                .file_name("video.mp4")
                .mime_str("video/mp4")?,
        );

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Video description failed: {}", error_text);
            return Err(format!("Video description failed: {}", error_text).into());
        }

        let result: VideoDescribeResponse = response.json().await?;
        info!("Video described: {}", result.description);
        Ok(result.description)
    }
}
