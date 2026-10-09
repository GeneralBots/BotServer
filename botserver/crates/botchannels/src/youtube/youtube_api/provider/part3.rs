use super::*;

impl YouTubeProvider {
    pub async fn get_comments(
        &self,
        access_token: &str,
        video_id: &str,
        page_token: Option<&str>,
        max_results: Option<u32>,
    ) -> Result<CommentThreadListResponse, ChannelError> {
        let url = format!("{}/commentThreads", self.api_base_url);

        let mut query_params = vec![
            ("part", "snippet,replies"),
            ("videoId", video_id),
        ];

        let max_results_str = max_results.unwrap_or(20).to_string();
        query_params.push(("maxResults", &max_results_str));

        if let Some(token) = page_token {
            query_params.push(("pageToken", token));
        }

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&query_params)
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

    /// Get channel analytics (requires YouTube Analytics API)
    pub async fn get_analytics(
        &self,
        access_token: &str,
        options: &AnalyticsRequest,
    ) -> Result<AnalyticsResponse, ChannelError> {
        let url = "https://youtubeanalytics.googleapis.com/v2/reports";

        let metrics = options
            .metrics
            .as_deref()
            .unwrap_or("views,estimatedMinutesWatched,averageViewDuration,subscribersGained");

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[
                ("ids", format!("channel=={}", options.channel_id).as_str()),
                ("startDate", &options.start_date),
                ("endDate", &options.end_date),
                ("metrics", metrics),
                ("dimensions", options.dimensions.as_deref().unwrap_or("day")),
            ])
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

    /// Refresh OAuth token
    pub async fn refresh_oauth_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<OAuthTokenResponse, ChannelError> {
        let url = format!("{}/token", self.oauth_base_url);

        let response = self
            .client
            .post(&url)
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
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

    /// Subscribe to a channel
    pub async fn subscribe(
        &self,
        access_token: &str,
        channel_id: &str,
    ) -> Result<Subscription, ChannelError> {
        let url = format!("{}/subscriptions", self.api_base_url);

        let request_body = serde_json::json!({
            "snippet": {
                "resourceId": {
                    "kind": "youtube#channel",
                    "channelId": channel_id
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

    /// Create a live broadcast
    pub async fn create_live_broadcast(
        &self,
        access_token: &str,
        broadcast: &LiveBroadcastRequest,
    ) -> Result<LiveBroadcast, ChannelError> {
        let url = format!("{}/liveBroadcasts", self.api_base_url);

        let request_body = serde_json::json!({
            "snippet": {
                "title": broadcast.title,
                "description": broadcast.description,
                "scheduledStartTime": broadcast.scheduled_start_time
            },
            "status": {
                "privacyStatus": broadcast.privacy_status
            },
            "contentDetails": {
                "enableAutoStart": broadcast.enable_auto_start,
                "enableAutoStop": broadcast.enable_auto_stop,
                "enableDvr": broadcast.enable_dvr,
                "enableEmbed": broadcast.enable_embed,
                "recordFromStart": broadcast.record_from_start
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .query(&[("part", "snippet,status,contentDetails")])
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
