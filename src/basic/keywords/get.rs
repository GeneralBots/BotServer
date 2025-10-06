use log::info;

use crate::shared::state::AppState;
use reqwest::{self, Client};
use rhai::{Dynamic, Engine};
use scraper::{Html, Selector};
use std::error::Error;

pub fn get_keyword(_state: &AppState, engine: &mut Engine) {
    let _ = engine.register_custom_syntax(
        &["GET", "$expr$"],
        false, // Expression, not statement
        move |context, inputs| {
            let url = context.eval_expression_tree(&inputs[0])?;
            let url_str = url.to_string();

            // Prevent path traversal attacks
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
                // Handle file:// URLs
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
    );
}

pub async fn execute_get(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    info!("Starting execute_get with URL: {}", url);

    // Create a client that ignores invalid certificates
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let response = client.get(url).send().await?;
    let html_content = response.text().await?;

    // Parse HTML and extract text only if it appears to be HTML
    if html_content.trim_start().starts_with("<!DOCTYPE html")
        || html_content.trim_start().starts_with("<html")
    {
        let document = Html::parse_document(&html_content);
        let selector = Selector::parse("body").unwrap_or_else(|_| Selector::parse("*").unwrap());

        let text_content = document
            .select(&selector)
            .flat_map(|element| element.text())
            .collect::<Vec<_>>()
            .join(" ");

        // Clean up the text
        let cleaned_text = text_content
            .replace('\n', " ")
            .replace('\t', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        Ok(cleaned_text)
    } else {
        Ok(html_content) // Return plain content as is if not HTML
    }
}
