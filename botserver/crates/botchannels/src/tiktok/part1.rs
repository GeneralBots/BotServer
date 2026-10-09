use super::*;

impl TikTokProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_base_url: "https://open.tiktokapis.com/v2".to_string(),
            oauth_base_url: "https://open.tiktokapis.com/v2/oauth".to_string(),
        }
    }

    /// Initialize a video upload (Direct Post)
    /// Returns upload URL and publish_id for tracking
    pub async fn init_video_upload(
        &self,
        access_token: &str,
        request: &VideoUploadRequest,
    ) -> Result<VideoUploadInit, ChannelError> {
        let url = format!("{}/post/publish/video/init/", self.api_base_url);

        let post_info = serde_json::json!({
            "title": request.title,
            "privacy_level": request.privacy_level,
            "disable_duet": request.disable_duet.unwrap_or(false),
            "disable_comment": request.disable_comment.unwrap_or(false),
            "disable_stitch": request.disable_stitch.unwrap_or(false),
            "video_cover_timestamp_ms": request.video_cover_timestamp_ms.unwrap_or(0),
            "brand_content_toggle": request.brand_content_toggle.unwrap_or(false),
            "brand_organic_toggle": request.brand_organic_toggle.unwrap_or(false)
        });

        let source_info = if let Some(url) = &request.video_url {
            serde_json::json!({
                "source": "PULL_FROM_URL",
                "video_url": url
            })
        } else {
            serde_json::json!({
                "source": "FILE_UPLOAD",
                "video_size": request.video_size.unwrap_or(0),
                "chunk_size": request.chunk_size.unwrap_or(10_000_000),
                "total_chunk_count": request.total_chunk_count.unwrap_or(1)
            })
        };

        let request_body = serde_json::json!({
            "post_info": post_info,
            "source_info": source_info
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let api_response: TikTokApiResponse<VideoUploadInit> =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        api_response.data.ok_or_else(|| ChannelError::ApiError {
            code: api_response.error.as_ref().map(|e| e.code.clone()),
            message: api_response
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "Unknown error".to_string()),
        })
    }

    /// Upload video chunk (for FILE_UPLOAD source)
    pub async fn upload_video_chunk(
        &self,
        upload_url: &str,
        chunk_data: &[u8],
        chunk_index: u32,
        total_chunks: u32,
    ) -> Result<(), ChannelError> {
        let content_range = if total_chunks == 1 {
            format!("bytes 0-{}/{}", chunk_data.len() - 1, chunk_data.len())
        } else {
            let start = chunk_index as usize * chunk_data.len();
            let end = start + chunk_data.len() - 1;
            let total = total_chunks as usize * chunk_data.len();
            format!("bytes {}-{}/{}", start, end, total)
        };

        let response = self
            .client
            .put(upload_url)
            .header("Content-Type", "video/mp4")
            .header("Content-Length", chunk_data.len().to_string())
            .header("Content-Range", content_range)
            .body(chunk_data.to_vec())
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        Ok(())
    }

    /// Check the status of a video upload/publish
    pub async fn get_publish_status(
        &self,
        access_token: &str,
        publish_id: &str,
    ) -> Result<PublishStatus, ChannelError> {
        let url = format!("{}/post/publish/status/fetch/", self.api_base_url);

        let request_body = serde_json::json!({
            "publish_id": publish_id
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let api_response: TikTokApiResponse<PublishStatus> =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        api_response.data.ok_or_else(|| ChannelError::ApiError {
            code: api_response.error.as_ref().map(|e| e.code.clone()),
            message: api_response
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "Unknown error".to_string()),
        })
    }

    /// Initialize a photo post
    pub async fn init_photo_post(
        &self,
        access_token: &str,
        request: &PhotoPostRequest,
    ) -> Result<PhotoUploadInit, ChannelError> {
        let url = format!("{}/post/publish/content/init/", self.api_base_url);

        let request_body = serde_json::json!({
            "post_info": {
                "title": request.title,
                "description": request.description,
                "privacy_level": request.privacy_level,
                "disable_comment": request.disable_comment.unwrap_or(false),
                "auto_add_music": request.auto_add_music.unwrap_or(true)
            },
            "source_info": {
                "source": "PULL_FROM_URL",
                "photo_cover_index": request.photo_cover_index.unwrap_or(0),
                "photo_images": request.photo_urls
            },
            "post_mode": "DIRECT_POST",
            "media_type": "PHOTO"
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let api_response: TikTokApiResponse<PhotoUploadInit> =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        api_response.data.ok_or_else(|| ChannelError::ApiError {
            code: api_response.error.as_ref().map(|e| e.code.clone()),
            message: api_response
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "Unknown error".to_string()),
        })
    }

    /// Get user info (requires user.info.basic or user.info.profile scope)
    pub async fn get_user_info(
        &self,
        access_token: &str,
        fields: &[&str],
    ) -> Result<TikTokUser, ChannelError> {
        let fields_param = fields.join(",");
        let url = format!("{}/user/info/", self.api_base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[("fields", fields_param)])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let api_response: TikTokApiResponse<UserInfoData> =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        api_response
            .data
            .and_then(|d| d.user)
            .ok_or_else(|| ChannelError::ApiError {
                code: api_response.error.as_ref().map(|e| e.code.clone()),
                message: api_response
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "User not found".to_string()),
            })
    }

    /// List user videos (requires video.list scope)
    pub async fn list_videos(
        &self,
        access_token: &str,
        options: &VideoListOptions,
    ) -> Result<VideoListResponse, ChannelError> {
        let url = format!("{}/video/list/", self.api_base_url);

        let fields = options.fields.as_deref().unwrap_or(
            "id,create_time,cover_image_url,share_url,video_description,duration,title",
        );

        let mut request_body = serde_json::json!({
            "max_count": options.max_count.unwrap_or(20)
        });

        if let Some(cursor) = &options.cursor {
            request_body["cursor"] = serde_json::json!(cursor);
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json; charset=UTF-8")
            .query(&[("fields", fields)])
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let api_response: TikTokApiResponse<VideoListResponse> =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        api_response.data.ok_or_else(|| ChannelError::ApiError {
            code: api_response.error.as_ref().map(|e| e.code.clone()),
            message: api_response
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "Failed to list videos".to_string()),
        })
    }
}
