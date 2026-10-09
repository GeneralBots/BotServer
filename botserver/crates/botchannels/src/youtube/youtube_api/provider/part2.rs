use super::*;

impl YouTubeProvider {
    pub async fn update_video(
        &self,
        access_token: &str,
        video_id: &str,
        update: &VideoUpdateRequest,
    ) -> Result<YouTubeVideo, ChannelError> {
        let url = format!("{}/videos", self.api_base_url);

        let update_body = serde_json::json!({
            "id": video_id,
            "snippet": {
                "title": update.title,
                "description": update.description,
                "tags": update.tags,
                "categoryId": update.category_id
            },
            "status": {
                "privacyStatus": update.privacy_status,
                "embeddable": update.embeddable,
                "publicStatsViewable": update.public_stats_viewable
            }
        });

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .query(&[("part", "snippet,status")])
            .json(&update_body)
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

    /// Delete a video
    pub async fn delete_video(
        &self,
        access_token: &str,
        video_id: &str,
    ) -> Result<(), ChannelError> {
        let url = format!("{}/videos", self.api_base_url);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[("id", video_id)])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 204 {
            return Ok(());
        }

        if !response.status().is_success() {
            return Err(parse_error_response(response).await);
        }

        Ok(())
    }

    /// Create a playlist
    pub async fn create_playlist(
        &self,
        access_token: &str,
        playlist: &PlaylistCreateRequest,
    ) -> Result<YouTubePlaylist, ChannelError> {
        let url = format!("{}/playlists", self.api_base_url);

        let request_body = serde_json::json!({
            "snippet": {
                "title": playlist.title,
                "description": playlist.description,
                "tags": playlist.tags,
                "defaultLanguage": playlist.default_language
            },
            "status": {
                "privacyStatus": playlist.privacy_status
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .query(&[("part", "snippet,status")])
            .json(&request_body)
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

    /// Add video to playlist
    pub async fn add_video_to_playlist(
        &self,
        access_token: &str,
        playlist_id: &str,
        video_id: &str,
        position: Option<u32>,
    ) -> Result<PlaylistItem, ChannelError> {
        let url = format!("{}/playlistItems", self.api_base_url);

        let mut request_body = serde_json::json!({
            "snippet": {
                "playlistId": playlist_id,
                "resourceId": {
                    "kind": "youtube#video",
                    "videoId": video_id
                }
            }
        });

        if let Some(pos) = position {
            request_body["snippet"]["position"] = serde_json::json!(pos);
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .query(&[("part", "snippet")])
            .json(&request_body)
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

    /// Remove video from playlist
    pub async fn remove_from_playlist(
        &self,
        access_token: &str,
        playlist_item_id: &str,
    ) -> Result<(), ChannelError> {
        let url = format!("{}/playlistItems", self.api_base_url);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[("id", playlist_item_id)])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 204 {
            return Ok(());
        }

        if !response.status().is_success() {
            return Err(parse_error_response(response).await);
        }

        Ok(())
    }

    /// Set video thumbnail
    pub async fn set_thumbnail(
        &self,
        access_token: &str,
        video_id: &str,
        image_data: &[u8],
        content_type: &str,
    ) -> Result<ThumbnailSetResponse, ChannelError> {
        let url = format!("{}/thumbnails/set", self.upload_base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", content_type)
            .query(&[("videoId", video_id)])
            .body(image_data.to_vec())
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

    /// Add a comment to a video
    pub async fn add_comment(
        &self,
        access_token: &str,
        video_id: &str,
        comment_text: &str,
    ) -> Result<CommentThread, ChannelError> {
        let url = format!("{}/commentThreads", self.api_base_url);

        let request_body = serde_json::json!({
            "snippet": {
                "videoId": video_id,
                "topLevelComment": {
                    "snippet": {
                        "textOriginal": comment_text
                    }
                }
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .query(&[("part", "snippet")])
            .json(&request_body)
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

    /// Reply to a comment
    pub async fn reply_to_comment(
        &self,
        access_token: &str,
        parent_id: &str,
        reply_text: &str,
    ) -> Result<Comment, ChannelError> {
        let url = format!("{}/comments", self.api_base_url);

        let request_body = serde_json::json!({
            "snippet": {
                "parentId": parent_id,
                "textOriginal": reply_text
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .query(&[("part", "snippet")])
            .json(&request_body)
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
}
