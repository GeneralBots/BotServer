use super::*;

impl PinterestProvider {
    async fn parse_error_response(&self, response: reqwest::Response) -> ChannelError {
        let status = response.status();

        if status.as_u16() == 401 {
            return ChannelError::AuthenticationFailed("Invalid or expired token".to_string());
        }

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());
            return ChannelError::RateLimited { retry_after };
        }

        let error_text = response.text().await.unwrap_or_default();

        if let Ok(error_response) = serde_json::from_str::<PinterestErrorResponse>(&error_text) {
            return ChannelError::ApiError {
                code: Some(error_response.code.to_string()),
                message: error_response.message,
            };
        }

        ChannelError::ApiError {
            code: Some(status.to_string()),
            message: error_text,
        }
    }
}
