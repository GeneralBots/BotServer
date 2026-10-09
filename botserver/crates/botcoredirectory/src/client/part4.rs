use super::*;

impl ZitadelClient {
    pub async fn search_users_by_metadata(&self, key: &str, value: &str) -> Result<Vec<serde_json::Value>> {
        let token = self.get_access_token().await?;
        let url = format!("{}/management/v1/users/_search", self.api_base);

        let body = serde_json::json!({
            "queries": [{
                "metadataQuery": {
                    "key": key,
                    "value": value,
                    "method": "TEXT_QUERY_METHOD_EQUALS"
                }
            }]
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to search users by metadata: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to search users by metadata: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse search response: {}", e))?;

        let users = data
            .get("result")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(users)
    }

    pub async fn find_or_create_user_by_phone(
        &self,
        phone: &str,
        first_name: &str,
        last_name: &str,
    ) -> Result<String> {
        let existing = self.search_users_by_phone(phone).await?;
        if let Some(user) = existing.first() {
            let user_id = user
                .get("userId")
                .or_else(|| user.get("id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("No userId in search result"))?
                .to_string();
            return Ok(user_id);
        }

        let username = format!("phone_{}", phone.replace('+', ""));
        let email = format!("{}@whatsapp.local", username);

        self.create_user_with_phone(
            &email,
            first_name,
            last_name,
            Some(&username),
            Some(phone),
            None,
        )
        .await
    }

    pub async fn get_user_memberships(
        &self,
        user_id: &str,
        offset: u32,
        limit: u32,
    ) -> Result<serde_json::Value> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/v2/users/{}/memberships?limit={}&offset={}",
            self.api_base, user_id, limit, offset
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get memberships: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get memberships: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse memberships response: {}", e))?;

        Ok(data)
    }

    pub async fn add_org_member(
        &self,
        org_id: &str,
        user_id: &str,
        roles: Vec<String>,
    ) -> Result<()> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/v2/organizations/{}/members",
            self.api_base, org_id
        );

        let body = serde_json::json!({
            "userId": user_id,
            "roles": roles
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to add org member: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to add org member: {}", error_text));
        }

        Ok(())
    }

    pub async fn remove_org_member(&self, org_id: &str, user_id: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/v2/organizations/{}/members/{}",
            self.api_base, org_id, user_id
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to remove org member: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to remove org member: {}", error_text));
        }

        Ok(())
    }

    pub async fn get_org_members(&self, org_id: &str) -> Result<Vec<serde_json::Value>> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/v2/organizations/{}/members",
            self.api_base, org_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get org members: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get org members: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse org members response: {}", e))?;

        let members = data
            .get("result")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(members)
    }

    pub async fn get_organization(&self, org_id: &str) -> Result<serde_json::Value> {
        let token = self.get_access_token().await?;
        let url = format!("{}/v2/organizations/{}", self.api_base, org_id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get organization: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get organization: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse organization response: {}", e))?;

        Ok(data)
    }

    pub async fn introspect_token(&self, token: &str) -> Result<serde_json::Value> {
        let url = format!("{}/oauth/v2/introspect", self.api_base);

        let params = [
            ("token", token),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        let response = self
            .http_client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to introspect token: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to introspect token: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse introspection response: {}", e))?;

        Ok(data)
    }

    pub async fn check_permission(
        &self,
        user_id: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool> {
        let token = self.get_access_token().await?;
        let url = format!("{}/v2/permissions/check", self.api_base);

        let check_payload = serde_json::json!({
            "userId": user_id,
            "permission": permission,
            "resource": resource,
            "namespace": self.config.project_id.clone()
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&check_payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to check permissions: {}", e))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let data: serde_json::Value = match response.json().await {
            Ok(d) => d,
            Err(e) => {
                log::warn!("check_permission: failed to parse response body: {}", e);
                return Ok(false);
            }
        };

        Ok(data.get("result").and_then(|r| r.as_bool()).unwrap_or(false))
    }
}
