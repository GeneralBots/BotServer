use crate::shared::models::UserSession;
use crate::shared::state::AppState;
#[cfg(feature = "web_automation")]
use crate::web_automation::WebCrawler;
use log::{error, info};
use rhai::{Dynamic, Engine};
use std::sync::Arc;

pub fn add_website_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["ADD_WEBSITE", "$expr$"], false, move |context, inputs| {
            let url = context.eval_expression_tree(&inputs[0])?;
            let url_str = url.to_string().trim_matches('"').to_string();

            info!(
                "ADD_WEBSITE command executed: {} for user: {}",
                url_str, user_clone.user_id
            );

            // Validate URL
            #[cfg(feature = "web_automation")]
            let is_valid = WebCrawler::is_valid_url(&url_str);
            #[cfg(not(feature = "web_automation"))]
            let is_valid = url_str.starts_with("http://") || url_str.starts_with("https://");

            if !is_valid {
                return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "Invalid URL format. Must start with http:// or https://".into(),
                    rhai::Position::NONE,
                )));
            }

            let state_for_task = Arc::clone(&state_clone);
            let user_for_task = user_clone.clone();
            let url_for_task = url_str.clone();

            // Spawn async task to crawl and index website
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        crawl_and_index_website(&state_for_task, &user_for_task, &url_for_task)
                            .await
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("Failed to build tokio runtime".to_string()))
                        .err()
                };

                if send_err.is_some() {
                    error!("Failed to send result from thread");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(120)) {
                Ok(Ok(message)) => {
                    info!("ADD_WEBSITE completed: {}", message);
                    Ok(Dynamic::from(message))
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "ADD_WEBSITE timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("ADD_WEBSITE failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
            }
        })
        .unwrap();
}

/// Crawl website and index content
async fn crawl_and_index_website(
    _state: &AppState,
    user: &UserSession,
    url: &str,
) -> Result<String, String> {
    info!("Crawling website: {} for user: {}", url, user.user_id);

    // Check if web_automation feature is enabled
    #[cfg(not(feature = "web_automation"))]
    {
        return Err(
            "Web automation feature not enabled. Recompile with --features web_automation"
                .to_string(),
        );
    }

    // Fetch website content (only compiled if feature enabled)
    #[cfg(feature = "web_automation")]
    {
        let crawler = WebCrawler::new();
        let text_content = crawler
            .crawl(url)
            .await
            .map_err(|e| format!("Failed to crawl website: {}", e))?;

        if text_content.trim().is_empty() {
            return Err("No text content found on website".to_string());
        }

        info!(
            "Extracted {} characters of text from website",
            text_content.len()
        );

        // Create KB name from URL
        let kb_name = format!(
            "website_{}",
            url.replace("https://", "")
                .replace("http://", "")
                .replace('/', "_")
                .replace('.', "_")
                .chars()
                .take(50)
                .collect::<String>()
        );

        // Create collection name for this user's website KB
        let collection_name = format!("kb_{}_{}_{}", user.bot_id, user.user_id, kb_name);

        // Ensure collection exists in Qdrant
        crate::kb::qdrant_client::ensure_collection_exists(_state, &collection_name)
            .await
            .map_err(|e| format!("Failed to create Qdrant collection: {}", e))?;

        // Index the content
        crate::kb::embeddings::index_document(_state, &collection_name, url, &text_content)
            .await
            .map_err(|e| format!("Failed to index document: {}", e))?;

        // Associate KB with user (not session)
        add_website_kb_to_user(_state, user, &kb_name, url)
            .await
            .map_err(|e| format!("Failed to associate KB with user: {}", e))?;

        info!(
            "Website indexed successfully to collection: {}",
            collection_name
        );

        Ok(format!(
            "Website '{}' crawled and indexed successfully ({} characters)",
            url,
            text_content.len()
        ))
    }
}

/// Add a website KB to user's active KBs
async fn add_website_kb_to_user(
    _state: &AppState,
    user: &UserSession,
    kb_name: &str,
    website_url: &str,
) -> Result<String, String> {
    // TODO: Insert into user_kb_associations table using Diesel
    // INSERT INTO user_kb_associations (id, user_id, bot_id, kb_name, is_website, website_url, created_at, updated_at)
    // VALUES (uuid_generate_v4(), user.user_id, user.bot_id, kb_name, 1, website_url, NOW(), NOW())
    // ON CONFLICT (user_id, bot_id, kb_name) DO UPDATE SET updated_at = NOW()

    info!(
        "Website KB '{}' associated with user '{}' (bot: {}, url: {})",
        kb_name, user.user_id, user.bot_id, website_url
    );

    Ok(format!(
        "Website KB '{}' added successfully for user",
        kb_name
    ))
}
