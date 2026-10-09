use super::*;

impl Default for SnapchatProvider {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait::async_trait]
impl ChannelProvider for SnapchatProvider {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Snapchat
    }

    fn max_text_length(&self) -> usize {
        34 // Headline character limit for ads
    }

    fn supports_images(&self) -> bool {
        true
    }

    fn supports_video(&self) -> bool {
        true
    }

    fn supports_links(&self) -> bool {
        true
    }

    async fn post(
        &self,
        account: &ChannelAccount,
        content: &PostContent,
    ) -> Result<PostResult, ChannelError> {
        let access_token = match &account.credentials {
            ChannelCredentials::OAuth { access_token, .. } => access_token.clone(),
            _ => {
                return Err(ChannelError::AuthenticationFailed(
                    "OAuth credentials required for Snapchat".to_string(),
                ))
            }
        };

        // Get required IDs from settings
        let ad_account_id = account
            .settings
            .custom
            .get("ad_account_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "ad_account_id required in settings".to_string(),
            })?;

        account
            .settings
            .custom
            .get("campaign_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "campaign_id required in settings".to_string(),
            })?;

        let ad_squad_id = account
            .settings
            .custom
            .get("ad_squad_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "ad_squad_id required in settings".to_string(),
            })?;

        let text = content.text.as_deref().unwrap_or("");

        // For Snapchat Ads, we need a media file first
        // This is a simplified flow - real implementation would handle media upload
        let media_id = content
            .metadata
            .get("media_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "media_id required for Snapchat ads".to_string(),
            })?;

        // Create creative
        let creative_request = CreativeCreateRequest {
            name: format!("Creative - {}", chrono::Utc::now().format("%Y%m%d%H%M%S")),
            creative_type: "SNAP_AD".to_string(),
            headline: Some(text.chars().take(34).collect()),
            brand_name: content
                .metadata
                .get("brand_name")
                .and_then(|v| v.as_str())
                .map(String::from),
            shareable: Some(true),
            call_to_action: content
                .metadata
                .get("cta")
                .and_then(|v| v.as_str())
                .map(String::from),
            top_snap_media_id: media_id.to_string(),
            top_snap_crop_position: Some("MIDDLE".to_string()),
            longform_video_properties: None,
            web_view_properties: content.link.as_ref().map(|url| WebViewProperties {
                url: url.clone(),
                allow_snap_javascript_sdk: Some(false),
                use_immersive_mode: Some(false),
                deep_link_urls: None,
            }),
        };

        let creative = self
            .create_creative(&access_token, ad_account_id, &creative_request)
            .await?;

        // Create ad
        let ad_request = AdCreateRequest {
            name: format!("Ad - {}", chrono::Utc::now().format("%Y%m%d%H%M%S")),
            status: "ACTIVE".to_string(),
            creative_id: creative.id.clone(),
            ad_type: "SNAP_AD".to_string(),
        };

        let ad = self
            .create_ad(&access_token, ad_squad_id, &ad_request)
            .await?;

        Ok(PostResult::success(ChannelType::Snapchat, ad.id, None))
    }

    async fn validate_credentials(
        &self,
        credentials: &ChannelCredentials,
    ) -> Result<bool, ChannelError> {
        match credentials {
            ChannelCredentials::OAuth { access_token, .. } => {
                match self.get_me(access_token).await {
                    Ok(_) => Ok(true),
                    Err(ChannelError::AuthenticationFailed(_)) => Ok(false),
                    Err(e) => Err(e),
                }
            }
            _ => Ok(false),
        }
    }

    async fn refresh_token(&self, account: &mut ChannelAccount) -> Result<(), ChannelError> {
        let (refresh_token, client_id, client_secret) = match &account.credentials {
            ChannelCredentials::OAuth { refresh_token, .. } => {
                let refresh = refresh_token.as_ref().ok_or_else(|| {
                    ChannelError::AuthenticationFailed("No refresh token available".to_string())
                })?;
                let client_id = account
                    .settings
                    .custom
                    .get("client_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ChannelError::AuthenticationFailed("Missing client_id".to_string())
                    })?;
                let client_secret = account
                    .settings
                    .custom
                    .get("client_secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ChannelError::AuthenticationFailed("Missing client_secret".to_string())
                    })?;
                (
                    refresh.clone(),
                    client_id.to_string(),
                    client_secret.to_string(),
                )
            }
            _ => {
                return Err(ChannelError::AuthenticationFailed(
                    "OAuth credentials required".to_string(),
                ))
            }
        };

        let token_response = self
            .refresh_oauth_token(&client_id, &client_secret, &refresh_token)
            .await?;

        let expires_at =
            chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64);

        account.credentials = ChannelCredentials::OAuth {
            access_token: token_response.access_token,
            refresh_token: Some(token_response.refresh_token),
            expires_at: Some(expires_at),
            scope: Some(token_response.scope),
        };

        Ok(())
    }
}

// ============================================================================
// Request Types
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignCreateRequest {
    pub name: String,
    pub status: String,
    pub objective: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub daily_budget_micro: Option<i64>,
    pub lifetime_spend_cap_micro: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdSquadCreateRequest {
    pub name: String,
    pub status: String,
    pub squad_type: String,
    pub placement: Option<PlacementV2>,
    pub billing_event: String,
    pub bid_micro: i64,
    pub daily_budget_micro: Option<i64>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub optimization_goal: String,
    pub targeting: Option<Targeting>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementV2 {
    pub config: String, // "AUTOMATIC" or "CUSTOM"
    pub platforms: Option<Vec<String>>,
    pub snap_positions: Option<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Targeting {
    pub geos: Option<Vec<GeoTarget>>,
    pub demographics: Option<Vec<Demographic>>,
    pub interests: Option<Vec<Interest>>,
    pub devices: Option<Vec<Device>>,
    pub segments: Option<Vec<SegmentTarget>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoTarget {
    pub country_code: String,
    pub region_id: Option<String>,
    pub metro_id: Option<String>,
    pub postal_code: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demographic {
    pub age_groups: Option<Vec<String>>,
    pub genders: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interest {
    pub category_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub os_type: Option<String>,
    pub os_version_min: Option<String>,
    pub os_version_max: Option<String>,
    pub make: Option<Vec<String>>,
    pub connection_type: Option<Vec<String>>,
    pub carrier: Option<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentTarget {
    pub segment_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeCreateRequest {
    pub name: String,
    pub creative_type: String,
    pub headline: Option<String>,
    pub brand_name: Option<String>,
    pub shareable: Option<bool>,
    pub call_to_action: Option<String>,
    pub top_snap_media_id: String,
    pub top_snap_crop_position: Option<String>,
    pub longform_video_properties: Option<LongformVideoProperties>,
    pub web_view_properties: Option<WebViewProperties>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongformVideoProperties {
    pub video_media_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebViewProperties {
    pub url: String,
    pub allow_snap_javascript_sdk: Option<bool>,
    pub use_immersive_mode: Option<bool>,
    pub deep_link_urls: Option<Vec<String>>,
}
