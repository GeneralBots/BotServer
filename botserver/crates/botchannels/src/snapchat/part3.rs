use super::*;

impl SnapchatProvider {
    pub async fn create_creative(
        &self,
        access_token: &str,
        ad_account_id: &str,
        creative: &CreativeCreateRequest,
    ) -> Result<Creative, ChannelError> {
        let url = format!(
            "{}/adaccounts/{}/creatives",
            Self::API_BASE, ad_account_id
        );

        let request_body = serde_json::json!({
            "creatives": [{
                "name": creative.name,
                "type": creative.creative_type,
                "headline": creative.headline,
                "brand_name": creative.brand_name,
                "shareable": creative.shareable,
                "call_to_action": creative.call_to_action,
                "top_snap_media_id": creative.top_snap_media_id,
                "top_snap_crop_position": creative.top_snap_crop_position,
                "longform_video_properties": creative.longform_video_properties,
                "web_view_properties": creative.web_view_properties
            }]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let result: CreativesResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        result
            .creatives
            .into_iter()
            .next()
            .and_then(|wrapper| wrapper.creative)
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "No creative in response".to_string(),
            })
    }

    /// Create an ad
    pub async fn create_ad(
        &self,
        access_token: &str,
        ad_squad_id: &str,
        ad: &AdCreateRequest,
    ) -> Result<Ad, ChannelError> {
        let url = format!("{}/adsquads/{}/ads", Self::API_BASE, ad_squad_id);

        let request_body = serde_json::json!({
            "ads": [{
                "name": ad.name,
                "status": ad.status,
                "creative_id": ad.creative_id,
                "type": ad.ad_type
            }]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let result: AdsResponse = response.json().await.map_err(|e| ChannelError::ApiError {
            code: None,
            message: e.to_string(),
        })?;

        result
            .ads
            .into_iter()
            .next()
            .and_then(|wrapper| wrapper.ad)
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "No ad in response".to_string(),
            })
    }

    /// Upload media (initialize upload)
    pub async fn init_media_upload(
        &self,
        access_token: &str,
        ad_account_id: &str,
        media: &MediaUploadRequest,
    ) -> Result<Media, ChannelError> {
        let url = format!("{}/adaccounts/{}/media", Self::API_BASE, ad_account_id);

        let request_body = serde_json::json!({
            "media": [{
                "name": media.name,
                "type": media.media_type,
                "ad_account_id": ad_account_id
            }]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let result: MediaResponse = response.json().await.map_err(|e| ChannelError::ApiError {
            code: None,
            message: e.to_string(),
        })?;

        result
            .media
            .into_iter()
            .next()
            .and_then(|wrapper| wrapper.media)
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "No media in response".to_string(),
            })
    }

    /// Upload media chunk
    pub async fn upload_media_chunk(
        &self,
        access_token: &str,
        ad_account_id: &str,
        media_id: &str,
        chunk_data: &[u8],
        chunk_number: u32,
    ) -> Result<(), ChannelError> {
        let url = format!(
            "{}/adaccounts/{}/media/{}/upload",
            Self::API_BASE, ad_account_id, media_id
        );

        let part = reqwest::multipart::Part::bytes(chunk_data.to_vec())
            .file_name("chunk")
            .mime_str("application/octet-stream")
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("chunk_number", chunk_number.to_string());

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .multipart(form)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        Ok(())
    }

    /// Complete media upload
    pub async fn complete_media_upload(
        &self,
        access_token: &str,
        ad_account_id: &str,
        media_id: &str,
        total_chunks: u32,
    ) -> Result<Media, ChannelError> {
        let url = format!(
            "{}/adaccounts/{}/media/{}/complete",
            Self::API_BASE, ad_account_id, media_id
        );

        let request_body = serde_json::json!({
            "total_chunks": total_chunks
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let result: SingleMediaResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        result.media.ok_or_else(|| ChannelError::ApiError {
            code: None,
            message: "No media in response".to_string(),
        })
    }

    /// Get media status
    pub async fn get_media(
        &self,
        access_token: &str,
        ad_account_id: &str,
        media_id: &str,
    ) -> Result<Media, ChannelError> {
        let url = format!(
            "{}/adaccounts/{}/media/{}",
            Self::API_BASE, ad_account_id, media_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let result: SingleMediaResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        result.media.ok_or_else(|| ChannelError::ApiError {
            code: None,
            message: "No media in response".to_string(),
        })
    }
}
