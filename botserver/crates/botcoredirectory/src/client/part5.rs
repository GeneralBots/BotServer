use super::*;

impl ZitadelClient {
    pub async fn set_user_password(&self, user_id: &str, password: &str, change_required: bool) -> Result<()> {
        let token = self.get_access_token().await?;
        let url = format!("{}/v2/users/{}/password", self.api_base, user_id);

        let body = serde_json::json!({
            "newPassword": {
                "password": password,
                "changeRequired": change_required
            }
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to set password: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to set password: {}", error_text));
        }

        Ok(())
    }

    pub async fn list_organizations(&self, limit: u32, offset: u32) -> Result<Vec<serde_json::Value>> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/management/v1/orgs/_search?limit={}&offset={}",
            self.api_base, limit, offset
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to list organizations: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to list organizations: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse organizations response: {}", e))?;

        let orgs = data
            .get("result")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(orgs)
    }

    pub async fn create_organization(&self, name: &str) -> Result<String> {
        let token = self.get_access_token().await?;
        // Note: Use management API (/management/v1/orgs) because this Zitadel build
        // does not expose /v2/organizations via HTTP.
        let url = format!("{}/management/v1/orgs", self.api_base);

        let body = serde_json::json!({ "name": name });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to create organization: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to create organization: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse organization response: {}", e))?;

        let org_id = data
            .get("organizationId")
            .or_else(|| data.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No organization ID in response"))?
            .to_string();

        Ok(org_id)
    }

    pub async fn update_organization_metadata(&self, org_id: &str, metadata: serde_json::Value) -> Result<()> {
        let token = self.get_access_token().await?;
        let url = format!("{}/v2/organizations/{}", self.api_base, org_id);

        let body = serde_json::json!({
            "metadata": metadata
        });

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to update organization: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to update organization: {}", error_text));
        }

        Ok(())
    }

    pub async fn create_pat(&self, user_id: &str, display_name: &str, expiration_date: Option<&str>) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/v2/users/{}/pat", self.api_base, user_id);

        let body = if let Some(expiry) = expiration_date {
            serde_json::json!({
                "displayName": display_name,
                "expirationDate": expiry
            })
        } else {
            serde_json::json!({
                "displayName": display_name
            })
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to create PAT: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to create PAT: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse PAT response: {}", e))?;

        let pat_token = data
            .get("token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("No token in PAT response"))?
            .to_string();

        Ok(pat_token)
    }
}
