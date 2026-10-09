use super::*;

impl BotModelsClient {
    pub async fn speech_to_text(
        &self,
        audio_url_or_path: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("BotModels is not enabled".into());
        }

        let url = format!("{}/api/speech/totext", self.config.base_url());
        trace!(
            "Converting speech to text at {}: {}",
            url,
            audio_url_or_path
        );

        let audio_data = if audio_url_or_path.starts_with("http") {
            let response = self.client.get(audio_url_or_path).send().await?;
            response.bytes().await?.to_vec()
        } else {
            tokio::fs::read(audio_url_or_path).await?
        };

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(audio_data)
                .file_name("audio.wav")
                .mime_str("audio/wav")?,
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
            log::error!("Speech to text failed: {}", error_text);
            return Err(format!("Speech to text failed: {}", error_text).into());
        }

        let result: SpeechToTextResponse = response.json().await?;
        info!("Speech converted: {}", result.text);
        Ok(result.text)
    }

    pub async fn scan_barcode(
        &self,
        image_url_or_path: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Err("BotModels is not enabled".into());
        }

        let url = format!("{}/api/vision/barcode", self.config.base_url());
        trace!("Scanning barcode at {}: {}", url, image_url_or_path);

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
            log::error!("Barcode scan failed: {}", error_text);
            return Err(format!("Barcode scan failed: {}", error_text).into());
        }

        let result: serde_json::Value = response.json().await?;
        info!("Barcode scanned: {:?}", result);
        Ok(result.to_string())
    }

    pub async fn health_check(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        let url = format!("{}/api/health", self.config.base_url());
        match self.client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn download_file(
        &self,
        url: &str,
        local_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client.get(url).send().await?;
        let bytes = response.bytes().await?;
        tokio::fs::write(local_path, bytes).await?;
        Ok(())
    }
}
