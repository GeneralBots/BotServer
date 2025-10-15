use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::{error, info};
use reqwest::{self, Client};
use rhai::{Dynamic, Engine};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

pub fn get_keyword(state: Arc<AppState>, _user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);

    engine
        .register_custom_syntax(&["GET", "$expr$"], false, move |context, inputs| {
            // Evaluate the URL expression
            let url = context.eval_expression_tree(&inputs[0])?;
            let url_str = url.to_string();

            info!("GET command executed: {}", url_str);

            // Enhanced security check for path traversal
            if !is_safe_path(&url_str) {
                return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "URL contains invalid or unsafe path sequences".into(),
                    rhai::Position::NONE,
                )));
            }

            let state_for_async = Arc::clone(&state_clone);
            let url_for_async = url_str.clone();

            // Create a channel to communicate the result back
            let (tx, rx) = tokio::sync::oneshot::channel();

            // Spawn the async task without blocking
            tokio::spawn(async move {
                log::trace!("Async task started for GET operation: {}", url_for_async);

                let result = if url_for_async.starts_with("https://")
                    || url_for_async.starts_with("http://")
                {
                    info!("HTTP(S) GET request: {}", url_for_async);
                    execute_get(&url_for_async).await
                } else {
                    info!("Local file GET request from bucket: {}", url_for_async);
                    get_from_bucket(&state_for_async, &url_for_async).await
                };

                // Send the result back through the channel
                let _ = tx.send(result);
            });

            // Block on receiving the result from the channel.
            // This is acceptable because we're in a custom syntax handler.
            let result = match futures::executor::block_on(rx) {
                Ok(inner) => inner.map_err(|e| {
                    Box::new(rhai::EvalAltResult::ErrorRuntime(
                        e.to_string().into(),
                        rhai::Position::NONE,
                    ))
                })?,
                Err(_) => {
                    return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "Failed to receive result from async task".into(),
                        rhai::Position::NONE,
                    )));
                }
            };

            Ok(Dynamic::from(result))
        })
        .unwrap();
}

/// Enhanced security check for path traversal and unsafe paths
fn is_safe_path(path: &str) -> bool {
    // Allow full URLs
    if path.starts_with("https://") || path.starts_with("http://") {
        return true;
    }

    // Check for various path traversal patterns
    if path.contains("..") {
        return false;
    }

    // Reject absolute paths (starting with /)
    if path.starts_with('/') {
        return false;
    }

    // Reject Windows-style absolute paths
    if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        return false;
    }

    // Normalize and validate the path doesn't escape
    if let Ok(normalized) = Path::new(path).canonicalize() {
        // If canonicalize succeeds, verify it doesn't contain parent directory references
        if normalized.to_string_lossy().contains("..") {
            return false;
        }
    }

    true
}

pub async fn execute_get(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    log::trace!("Starting execute_get with URL: {}", url);

    // Build secure HTTP client (removed danger_accept_invalid_certs)
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| {
            error!("Failed to build HTTP client: {}", e);
            e
        })?;

    let response = client.get(url).send().await.map_err(|e| {
        error!("HTTP request failed for URL {}: {}", url, e);
        e
    })?;

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        error!(
            "HTTP request returned non-success status for URL {}: {}",
            url, status
        );
        return Err(format!("HTTP request failed with status: {}", status).into());
    }

    let content = response.text().await.map_err(|e| {
        error!("Failed to read response text for URL {}: {}", url, e);
        e
    })?;

    log::trace!(
        "Successfully executed GET request for URL: {}, content length: {}",
        url,
        content.len()
    );
    Ok(content)
}

pub async fn get_from_bucket(
    state: &AppState,
    file_path: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    log::trace!("Getting file from bucket: {}", file_path);

    // Additional validation for file path
    if !is_safe_path(file_path) {
        error!("Unsafe file path detected: {}", file_path);
        return Err("Invalid file path".into());
    }

    // Ensure the S3 client is configured
    let s3_client = match &state.s3_client {
        Some(client) => client,
        None => {
            error!(
                "S3 client not configured when trying to get file: {}",
                file_path
            );
            return Err("S3 client not configured".into());
        }
    };

    // Resolve the bucket name safely, handling missing configuration values
    let bucket_name = {
        let cfg = state
            .config
            .as_ref()
            .ok_or_else(|| -> Box<dyn Error + Send + Sync> {
                "App configuration missing".into()
            })?;

        let org_prefix = &cfg.minio.org_prefix;

        // Validate org_prefix doesn't contain suspicious characters
        if org_prefix.contains("..") || org_prefix.contains('/') {
            error!("Invalid org_prefix in configuration: {}", org_prefix);
            return Err("Invalid organization prefix in configuration".into());
        }

        format!("{}.default.gbai", org_prefix)
    };

    log::trace!("Using bucket: {} for file: {}", bucket_name, file_path);

    // Perform the S3 GetObject request
    let response = s3_client
        .get_object()
        .bucket(&bucket_name)
        .key(file_path)
        .send()
        .await
        .map_err(|e| {
            error!(
                "S3 get_object failed for bucket {} key {}: {}",
                bucket_name, file_path, e
            );
            e
        })?;

    // Collect the body bytes
    let data = response.body.collect().await.map_err(|e| {
        error!(
            "Failed to collect S3 response body for bucket {} key {}: {}",
            bucket_name, file_path, e
        );
        e
    })?;

    // Handle PDF files specially; otherwise treat as UTF‑8 text
    let bytes = data.into_bytes().to_vec();

    let content = if file_path.to_ascii_lowercase().ends_with(".pdf") {
        // Extract text from PDF using the `pdf_extract` crate
        match pdf_extract::extract_text_from_mem(&bytes) {
            Ok(text) => text,
            Err(e) => {
                error!(
                    "Failed to extract text from PDF for bucket {} key {}: {}",
                    bucket_name, file_path, e
                );
                return Err(format!("PDF extraction failed: {}", e).into());
            }
        }
    } else {
        // Convert bytes to a UTF‑8 String
        String::from_utf8(bytes).map_err(|e| {
            error!(
                "Failed to convert S3 response to UTF-8 for bucket {} key {}: {}",
                bucket_name, file_path, e
            );
            e
        })?
    };

    log::trace!(
        "Successfully retrieved file from bucket: {}, content length: {}",
        file_path,
        content.len()
    );
    Ok(content)
}
