use super::*;

pub(super) fn build_blocking_client(auth: &AuthConfig) -> Result<reqwest::blocking::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();

    match &auth.auth_type {
        AuthType::ApiKey => {
            let header_name = auth.api_key_header.as_deref().unwrap_or("X-API-Key");
            if let Some(ref key) = auth.api_key {
                headers.insert(
                    reqwest::header::HeaderName::from_bytes(header_name.as_bytes())
                        .map_err(|e| format!("Invalid header name: {e}"))?,
                    reqwest::header::HeaderValue::from_str(key)
                        .map_err(|e| format!("Invalid header value: {e}"))?,
                );
            }
        }
        AuthType::Bearer => {
            if let Some(ref key) = auth.api_key {
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {key}"))
                        .map_err(|e| format!("Invalid bearer token: {e}"))?,
                );
            }
        }
        AuthType::Basic => {
            if let (Some(ref user), Some(ref pass)) = (&auth.username, &auth.password) {
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("{user}:{pass}"));
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Basic {encoded}"))
                        .map_err(|e| format!("Invalid basic auth: {e}"))?,
                );
            }
        }
        AuthType::OAuth2 | AuthType::None | AuthType::Custom(_) => {}
    }

    if let Some(ref extra) = auth.extra_headers {
        for (name, value) in extra {
            headers.insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|e| format!("Invalid header name: {e}"))?,
                reqwest::header::HeaderValue::from_str(value)
                    .map_err(|e| format!("Invalid header value: {e}"))?,
            );
        }
    }

    reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))
}
