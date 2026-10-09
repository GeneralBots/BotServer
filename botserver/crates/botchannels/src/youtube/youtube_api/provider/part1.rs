use super::*;

impl YouTubeProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_base_url: "https://www.googleapis.com/youtube/v3".to_string(),
            upload_base_url: "https://www.googleapis.com/upload/youtube/v3".to_string(),
            oauth_base_url: "https://oauth2.googleapis.com".to_string(),
        }
    }

    /// Upload a video to YouTube
    pub async fn upload_video(
        &self,
        access_token: &str,
        video: &VideoUploadRequest,
        video_data: &[u8],
    ) -> Result<YouTubeVideo, ChannelError> {
        // Step 1: Initialize resumable upload
        let init_url = format!(
            "{}/videos?uploadType=resumable&part=snippet,status,contentDetails",
            self.upload_base_url
        );

        let metadata = VideoMetadata::from_request(video);

        let init_response = self
            .client
            .post(&init_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("X-Upload-Content-Type", &video.content_type)
            .header("X-Upload-Content-Length", video_data.len().to_string())
            .json(&metadata)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !init_response.status().is_success() {
            return Err(parse_error_response(init_response).await);
        }

        let upload_url = init_response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "Missing upload URL in response".to_string(),
            })?
            .to_string();

        // Step 2: Upload video data
        let upload_response = self
            .client
            .put(&upload_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", &video.content_type)
            .header("Content-Length", video_data.len().to_string())
            .body(video_data.to_vec())
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !upload_response.status().is_success() {
            return Err(parse_error_response(upload_response).await);
        }

        upload_response
            .json::<YouTubeVideo>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Create a community post (text, poll, image, or video)
    pub async fn create_community_post(
        &self,
        access_token: &str,
        post: &CommunityPostRequest,
    ) -> Result<CommunityPost, ChannelError> {
        // Note: Community Posts API is limited and may require additional permissions
        let url = format!("{}/activities", self.api_base_url);

        let request_body = serde_json::json!({
            "snippet": {
                "description": post.text,
                "channelId": post.channel_id
            },
            "contentDetails": {
                "bulletin": {
                    "resourceId": post.attached_video_id.as_ref().map(|vid| {
                        serde_json::json!({
                            "kind": "youtube#video",
                            "videoId": vid
                        })
                    })
                }
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .query(&[("part", "snippet,contentDetails")])
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(parse_error_response(response).await);
        }

        response
            .json::<CommunityPost>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Get channel information
    pub async fn get_channel(&self, access_token: &str) -> Result<YouTubeChannel, ChannelError> {
        let url = format!("{}/channels", self.api_base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[
                ("part", "snippet,contentDetails,statistics,status,brandingSettings"),
                ("mine", "true"),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(parse_error_response(response).await);
        }

        let list_response: ChannelListResponse = response.json().await.map_err(|e| {
            ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            }
        })?;

        list_response.items.into_iter().next().ok_or_else(|| {
            ChannelError::ApiError {
                code: None,
                message: "No channel found".to_string(),
            }
        })
    }

    /// Get channel by ID
    pub async fn get_channel_by_id(
        &self,
        access_token: &str,
        channel_id: &str,
    ) -> Result<YouTubeChannel, ChannelError> {
        let url = format!("{}/channels", self.api_base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[
                ("part", "snippet,contentDetails,statistics,status"),
                ("id", channel_id),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(parse_error_response(response).await);
        }

        let list_response: ChannelListResponse = response.json().await.map_err(|e| {
            ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            }
        })?;

        list_response.items.into_iter().next().ok_or_else(|| {
            ChannelError::ApiError {
                code: None,
                message: "Channel not found".to_string(),
            }
        })
    }

    /// List videos from a channel or playlist
    pub async fn list_videos(
        &self,
        access_token: &str,
        options: &VideoListOptions,
    ) -> Result<VideoListResponse, ChannelError> {
        let url = format!("{}/search", self.api_base_url);

        let mut query_params = vec![
            ("part", "snippet".to_string()),
            ("type", "video".to_string()),
            ("maxResults", options.max_results.unwrap_or(25).to_string()),
        ];

        if let Some(channel_id) = &options.channel_id {
            query_params.push(("channelId", channel_id.clone()));
        }

        if options.for_mine.unwrap_or(false) {
            query_params.push(("forMine", "true".to_string()));
        }

        if let Some(order) = &options.order {
            query_params.push(("order", order.clone()));
        }

        if let Some(page_token) = &options.page_token {
            query_params.push(("pageToken", page_token.clone()));
        }

        if let Some(published_after) = &options.published_after {
            query_params.push(("publishedAfter", published_after.clone()));
        }

        if let Some(published_before) = &options.published_before {
            query_params.push(("publishedBefore", published_before.clone()));
        }

        let query_refs: Vec<(&str, &str)> = query_params
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&query_refs)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(parse_error_response(response).await);
        }

        response.json().await.map_err(|e| ChannelError::ApiError {
            code: None,
            message: e.to_string(),
        })
    }

    /// Get video details by ID
    pub async fn get_video(
        &self,
        access_token: &str,
        video_id: &str,
    ) -> Result<YouTubeVideo, ChannelError> {
        let url = format!("{}/videos", self.api_base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[
                ("part", "snippet,contentDetails,statistics,status,player"),
                ("id", video_id),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(parse_error_response(response).await);
        }

        let list_response: YouTubeVideoListResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        list_response.items.into_iter().next().ok_or_else(|| {
            ChannelError::ApiError {
                code: None,
                message: "Video not found".to_string(),
            }
        })
    }
}
