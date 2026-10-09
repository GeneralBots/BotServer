use super::*;

pub(super) async fn get_oauth_token(
    http_client: &reqwest::Client,
    auth_service: &dyn botlib::traits::AuthServiceTrait,
) -> Result<String, String> {
    let token_url = format!("{}/oauth/v2/token", auth_service.api_url());

    let params = [
        ("grant_type", "client_credentials".to_string()),
        ("client_id", auth_service.client_id().to_string()),
        ("client_secret", auth_service.client_secret().to_string()),
        ("scope", "openid profile email urn:zitadel:iam:org:project:id:zitadel:aud".to_string()),
    ];

    let response = http_client
        .post(&token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to request OAuth token: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("OAuth token request failed: {}", error_text));
    }

    let token_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OAuth token response: {}", e))?;

    let access_token = token_data
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "No access_token in OAuth response".to_string())?
        .to_string();

    info!("Successfully obtained OAuth access token via client credentials");
    Ok(access_token)
}
