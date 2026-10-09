use super::*;

impl SnapchatProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Get authenticated user info
    pub async fn get_me(&self, access_token: &str) -> Result<SnapchatUser, ChannelError> {
        let url = format!("{}/me", Self::API_BASE);

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

        let result: MeResponse = response.json().await.map_err(|e| ChannelError::ApiError {
            code: None,
            message: e.to_string(),
        })?;

        result.me.ok_or_else(|| ChannelError::ApiError {
            code: None,
            message: "No user data in response".to_string(),
        })
    }

    /// List organizations for the authenticated user
    pub async fn list_organizations(
        &self,
        access_token: &str,
    ) -> Result<Vec<Organization>, ChannelError> {
        let url = format!("{}/me/organizations", Self::API_BASE);

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

        let result: OrganizationsResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        Ok(result
            .organizations
            .into_iter()
            .filter_map(|wrapper| wrapper.organization)
            .collect())
    }

    /// List ad accounts for an organization
    pub async fn list_ad_accounts(
        &self,
        access_token: &str,
        organization_id: &str,
    ) -> Result<Vec<AdAccount>, ChannelError> {
        let url = format!(
            "{}/organizations/{}/adaccounts",
            Self::API_BASE, organization_id
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

        let result: AdAccountsResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        Ok(result
            .adaccounts
            .into_iter()
            .filter_map(|wrapper| wrapper.adaccount)
            .collect())
    }

    /// Create a campaign
    pub async fn create_campaign(
        &self,
        access_token: &str,
        ad_account_id: &str,
        campaign: &CampaignCreateRequest,
    ) -> Result<Campaign, ChannelError> {
        let url = format!(
            "{}/adaccounts/{}/campaigns",
            Self::API_BASE, ad_account_id
        );

        let request_body = serde_json::json!({
            "campaigns": [{
                "name": campaign.name,
                "status": campaign.status,
                "objective": campaign.objective,
                "start_time": campaign.start_time,
                "end_time": campaign.end_time,
                "daily_budget_micro": campaign.daily_budget_micro,
                "lifetime_spend_cap_micro": campaign.lifetime_spend_cap_micro
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

        let result: CampaignsResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        result
            .campaigns
            .into_iter()
            .next()
            .and_then(|wrapper| wrapper.campaign)
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "No campaign in response".to_string(),
            })
    }

    /// List campaigns for an ad account
    pub async fn list_campaigns(
        &self,
        access_token: &str,
        ad_account_id: &str,
    ) -> Result<Vec<Campaign>, ChannelError> {
        let url = format!(
            "{}/adaccounts/{}/campaigns",
            Self::API_BASE, ad_account_id
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

        let result: CampaignsResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        Ok(result
            .campaigns
            .into_iter()
            .filter_map(|wrapper| wrapper.campaign)
            .collect())
    }

    /// Create an ad squad (ad set)
    pub async fn create_ad_squad(
        &self,
        access_token: &str,
        campaign_id: &str,
        ad_squad: &AdSquadCreateRequest,
    ) -> Result<AdSquad, ChannelError> {
        let url = format!("{}/campaigns/{}/adsquads", Self::API_BASE, campaign_id);

        let request_body = serde_json::json!({
            "adsquads": [{
                "name": ad_squad.name,
                "status": ad_squad.status,
                "type": ad_squad.squad_type,
                "placement_v2": ad_squad.placement,
                "billing_event": ad_squad.billing_event,
                "bid_micro": ad_squad.bid_micro,
                "daily_budget_micro": ad_squad.daily_budget_micro,
                "start_time": ad_squad.start_time,
                "end_time": ad_squad.end_time,
                "optimization_goal": ad_squad.optimization_goal,
                "targeting": ad_squad.targeting
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

        let result: AdSquadsResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        result
            .adsquads
            .into_iter()
            .next()
            .and_then(|wrapper| wrapper.adsquad)
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "No ad squad in response".to_string(),
            })
    }
}
