use super::*;

impl ZitadelClient {
    pub async fn create_user_with_phone(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        username: Option<&str>,
        phone: Option<&str>,
        initial_password: Option<&str>,
    ) -> Result<String> {
        let token = self.get_access_token().await?;
        // Note: This Zitadel build (Jan 2024) does not expose /v2/users/human via HTTP.
        // Use _import (not AddHuman): this build silently DROPS the create-time
        // password on AddHuman, leaving the user uninitialized (init code, no
        // working hash). _import persists a working hash in the same call.
        // Same payload shape as AddHuman; legacy builds without _import fall
        // back below.
        let url = format!("{}/management/v1/users/human/_import", self.api_base);

        let mut body = serde_json::json!({
            "userName": username.unwrap_or(email),
            "profile": {
                "firstName": first_name,
                "lastName": last_name,
                "displayName": format!("{} {}", first_name, last_name)
            },
            "email": {
                "email": email,
                // Zitadel v1 proto field is isEmailVerified — "isVerified" is
                // silently dropped, leaving the user uninitialized with an init
                // code so later password sets fail with COMMAND-M9dse (#1365).
                "isEmailVerified": true
            }
        });

        if let Some(phone_number) = phone {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("phone".to_string(), serde_json::json!({
                    "phone": phone_number,
                    "isPhoneVerified": true
                }));
            }
        }

        if let Some(password) = initial_password {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("password".to_string(), serde_json::Value::String(password.to_string()));
            }
        }

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to create user: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            // Legacy Zitadel builds without the _import endpoint: retry the
            // classic AddHuman route so user creation still works there (the
            // password flag is dropped on those builds — caller must set the
            // password separately).
            if status == reqwest::StatusCode::NOT_FOUND && url.ends_with("/_import") {
                let fallback_url = url.trim_end_matches("/_import").to_string();
                let retry = self
                    .http_client
                    .post(&fallback_url)
                    .bearer_auth(&token)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| anyhow!("Failed to create user: {}", e))?;
                if !retry.status().is_success() {
                    let retry_text = retry.text().await.unwrap_or_default();
                    return Err(anyhow!("Failed to create user: {}", retry_text));
                }
                let user_data: serde_json::Value = retry
                    .json()
                    .await
                    .map_err(|e| anyhow!("Failed to parse user response: {}", e))?;
                let user_id = user_data
                    .get("userId")
                    .and_then(|id| id.as_str())
                    .ok_or_else(|| anyhow!("No userId in response"))?
                    .to_string();
                return Ok(user_id);
            }
            return Err(anyhow!("Failed to create user: {}", error_text));
        }

        let user_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse user response: {}", e))?;

        let user_id = user_data
            .get("userId")
            .and_then(|id| id.as_str())
            .ok_or_else(|| anyhow!("No userId in response"))?
            .to_string();

        Ok(user_id)
    }

    pub async fn create_user_with_password(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        username: Option<&str>,
        initial_password: Option<&str>,
    ) -> Result<String> {
        self.create_user_with_phone(email, first_name, last_name, username, None, initial_password).await
    }

    pub async fn get_user(&self, user_id: &str) -> Result<serde_json::Value> {
        let token = self.get_access_token().await?;
        let url = format!("{}/v2/users/{}", self.api_base, user_id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get user: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get user: {}", error_text));
        }

        let user_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse user response: {}", e))?;

        Ok(user_data)
    }

    pub async fn list_users(&self, limit: u32, offset: u32) -> Result<Vec<serde_json::Value>> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/management/v1/users/_search?limit={}&offset={}",
            self.api_base, limit, offset
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to list users: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to list users: {}", error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse users response: {}", e))?;

        let users = data
            .get("result")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(users)
    }

    pub async fn search_users(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        let token = self.get_access_token().await?;
        let url = format!("{}/management/v1/users/_search", self.api_base);

        let body = serde_json::json!({
            "queries": [{
                "userNameQuery": {
                    "userName": query,
                    "method": "TEXT_QUERY_METHOD_CONTAINS_IGNORE_CASE"
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
            .map_err(|e| anyhow!("Failed to search users: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to search users: {}", error_text));
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

    pub async fn search_users_by_phone(&self, phone: &str) -> Result<Vec<serde_json::Value>> {
        let token = self.get_access_token().await?;
        let url = format!("{}/management/v1/users/_search", self.api_base);

        let body = serde_json::json!({
            "queries": [{
                "phoneQuery": {
                    "phone": phone,
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
            .map_err(|e| anyhow!("Failed to search users by phone: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to search users by phone: {}", error_text));
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

    pub async fn search_users_by_email(&self, email: &str) -> Result<Vec<serde_json::Value>> {
        let token = self.get_access_token().await?;
        let url = format!("{}/management/v1/users/_search", self.api_base);

        let body = serde_json::json!({
            "queries": [{
                "emailQuery": {
                    "email": email,
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
            .map_err(|e| anyhow!("Failed to search users by email: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to search users by email: {}", error_text));
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
}
