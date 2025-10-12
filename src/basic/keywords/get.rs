use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::info;
use reqwest::{self, Client};
use rhai::{Dynamic, Engine};
use std::error::Error;

pub fn get_keyword(state: &AppState, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(&["GET", "$expr$"], false, move |context, inputs| {
            let url = context.eval_expression_tree(&inputs[0])?;
            let url_str = url.to_string();

            if url_str.contains("..") {
                return Err("URL contains invalid path traversal sequences like '..'.".into());
            }

            let state_for_async = state_clone.clone();
            let url_for_async = url_str.clone();

            if url_str.starts_with("https://") {
                info!("HTTPS GET request: {}", url_for_async);

                let fut = execute_get(&url_for_async);
                let result =
                    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
                        .map_err(|e| format!("HTTP request failed: {}", e))?;

                Ok(Dynamic::from(result))
            } else {
                info!("Local file GET request from bucket: {}", url_for_async);

                let fut = get_from_bucket(&state_for_async, &url_for_async);
                let result =
                    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
                        .map_err(|e| format!("Bucket GET failed: {}", e))?;

                Ok(Dynamic::from(result))
            }
        })
        .unwrap();
}

pub async fn execute_get(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    info!("Starting execute_get with URL: {}", url);

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let response = client.get(url).send().await?;
    let content = response.text().await?;

    Ok(content)
}

pub async fn get_from_bucket(
    state: &AppState,
    file_path: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    info!("Getting file from bucket: {}", file_path);

    if let Some(s3_client) = &state.s3_client {
        let bucket_name =
            std::env::var("DEFAULT_BUCKET").unwrap_or_else(|_| "default-bucket".to_string());

        let response = s3_client
            .get_object()
            .bucket(&bucket_name)
            .key(file_path)
            .send()
            .await?;

        let data = response.body.collect().await?;
        let content = String::from_utf8(data.into_bytes().to_vec())?;

        Ok(content)
    } else {
        Err("S3 client not configured".into())
    }
}
