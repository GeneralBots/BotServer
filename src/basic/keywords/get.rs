use log::info;
use crate::shared::state::AppState;
use crate::shared::models::UserSession;
use reqwest::{self, Client};
use rhai::{Dynamic, Engine};
use std::error::Error;

pub fn get_keyword(_state: &AppState, _user: UserSession, engine: &mut Engine) {
    engine
        .register_custom_syntax(
            &["GET", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?;
                let url_str = url.to_string();

                if url_str.contains("..") {
                    return Err("URL contains invalid path traversal sequences like '..'.".into());
                }

                let modified_url = if url_str.starts_with("/") {
                    let work_root = std::env::var("WORK_ROOT").unwrap_or_else(|_| "./work".to_string());
                    let full_path = std::path::Path::new(&work_root)
                        .join(url_str.trim_start_matches('/'))
                        .to_string_lossy()
                        .into_owned();

                    let base_url = "file://";
                    format!("{}{}", base_url, full_path)
                } else {
                    url_str.to_string()
                };

                if modified_url.starts_with("https://") {
                    info!("HTTPS GET request: {}", modified_url);

                    let fut = execute_get(&modified_url);
                    let result =
                        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
                            .map_err(|e| format!("HTTP request failed: {}", e))?;

                    Ok(Dynamic::from(result))
                } else if modified_url.starts_with("file://") {
                    let file_path = modified_url.trim_start_matches("file://");
                    match std::fs::read_to_string(file_path) {
                        Ok(content) => Ok(Dynamic::from(content)),
                        Err(e) => Err(format!("Failed to read file: {}", e).into()),
                    }
                } else {
                    Err(
                        format!("GET request failed: URL must begin with 'https://' or 'file://'")
                            .into(),
                    )
                }
            },
        )
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
