use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::{debug, error, info, warn};
use reqwest::{self, Client};
use rhai::{Dynamic, Engine};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

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

            let state_for_blocking = Arc::clone(&state_clone);
            let url_for_blocking = url_str.clone();

            // Use spawn_blocking for synchronous execution of async operations
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    debug!("Starting GET operation: {}", url_for_blocking);

                    let result = if url_for_blocking.starts_with("https://")
                        || url_for_blocking.starts_with("http://")
                    {
                        info!("HTTP(S) GET request: {}", url_for_blocking);
                        execute_get(&url_for_blocking).await
                    } else {
                        info!("Local file GET request from bucket: {}", url_for_blocking);
                        get_from_bucket(&state_for_blocking, &url_for_blocking).await
                    };

                    debug!(
                        "GET operation completed for: {}, success: {}",
                        url_for_blocking,
                        result.is_ok()
                    );

                    result
                })
            });

            match result {
                Ok(content) => Ok(Dynamic::from(content)),
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.to_string().into(),
                    rhai::Position::NONE,
                ))),
            }
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

    // Additional checks for suspicious patterns
    if path.contains("//") || path.contains("~") || path.contains("*") || path.contains("?") {
        return false;
    }

    // For local file paths, ensure they don't try to escape
    if !path.starts_with("http") {
        let path_obj = Path::new(path);
        if path_obj.components().count()
            != path_obj
                .components()
                .filter(|c| matches!(c, std::path::Component::Normal(_)))
                .count()
        {
            return false;
        }
    }

    true
}

pub async fn execute_get(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    debug!("Starting execute_get with URL: {}", url);

    // Build secure HTTP client with reasonable timeouts
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .tcp_keepalive(Duration::from_secs(30))
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
        let error_body = response.text().await.unwrap_or_default();
        error!(
            "HTTP request returned non-success status for URL {}: {} - {}",
            url, status, error_body
        );
        return Err(format!(
            "HTTP request failed with status: {} - {}",
            status, error_body
        )
        .into());
    }

    let content = response.text().await.map_err(|e| {
        error!("Failed to read response text for URL {}: {}", url, e);
        e
    })?;

    debug!(
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
    debug!("Getting file from bucket: {}", file_path);

    // Additional validation for file path
    if !is_safe_path(file_path) {
        error!("Unsafe file path detected: {}", file_path);
        return Err("Invalid file path".into());
    }

    // Ensure the S3 client is configured
    let s3_client = match &state.s3_client {
        Some(client) => {
            debug!("S3 client is available");
            client
        }
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
                error!("App configuration missing");
                "App configuration missing".into()
            })?;

        let org_prefix = &cfg.minio.org_prefix;

        // Validate org_prefix doesn't contain suspicious characters
        if org_prefix.contains("..") || org_prefix.contains('/') || org_prefix.contains('\\') {
            error!("Invalid org_prefix in configuration: {}", org_prefix);
            return Err("Invalid organization prefix in configuration".into());
        }

        let bucket = format!("{}default.gbai", org_prefix);
        debug!("Resolved bucket name: {}", bucket);
        bucket
    };

    debug!("Using bucket: {} for file: {}", bucket_name, file_path);

    // Check if bucket exists first (optional but helpful for debugging)
    match s3_client.head_bucket().bucket(&bucket_name).send().await {
        Ok(_) => debug!("Bucket exists: {}", bucket_name),
        Err(e) => {
            error!(
                "Bucket does not exist or inaccessible: {} - {}",
                bucket_name, e
            );
            return Err(format!("Bucket inaccessible: {}", e).into());
        }
    }

    // Perform the S3 GetObject request with timeout
    let get_object_future = s3_client
        .get_object()
        .bucket(&bucket_name)
        .key(file_path)
        .send();

    let response = match tokio::time::timeout(Duration::from_secs(30), get_object_future).await {
        Ok(Ok(response)) => {
            debug!("S3 GetObject successful for key: {}", file_path);
            response
        }
        Ok(Err(e)) => {
            error!(
                "S3 get_object failed for bucket {} key {}: {}",
                bucket_name, file_path, e
            );
            return Err(format!("S3 operation failed: {}", e).into());
        }
        Err(_) => {
            error!(
                "S3 get_object timed out for bucket {} key {}",
                bucket_name, file_path
            );
            return Err("S3 operation timed out".into());
        }
    };

    // Collect the body bytes with timeout
    let body_future = response.body.collect();
    let data = match tokio::time::timeout(Duration::from_secs(30), body_future).await {
        Ok(Ok(data)) => {
            debug!(
                "Successfully collected S3 response body for key: {}",
                file_path
            );
            data
        }
        Ok(Err(e)) => {
            error!(
                "Failed to collect S3 response body for bucket {} key {}: {}",
                bucket_name, file_path, e
            );
            return Err(format!("Failed to read S3 response: {}", e).into());
        }
        Err(_) => {
            error!(
                "Timeout collecting S3 response body for bucket {} key {}",
                bucket_name, file_path
            );
            return Err("Timeout reading S3 response body".into());
        }
    };

    // Handle PDF files specially; otherwise treat as UTF‑8 text
    let bytes = data.into_bytes().to_vec();
    debug!(
        "Retrieved {} bytes from S3 for key: {}",
        bytes.len(),
        file_path
    );

    let content = if file_path.to_ascii_lowercase().ends_with(".pdf") {
        debug!("Processing as PDF file: {}", file_path);
        // Extract text from PDF using the `pdf_extract` crate
        match pdf_extract::extract_text_from_mem(&bytes) {
            Ok(text) => {
                debug!(
                    "Successfully extracted text from PDF, length: {}",
                    text.len()
                );
                text
            }
            Err(e) => {
                error!(
                    "Failed to extract text from PDF for bucket {} key {}: {}",
                    bucket_name, file_path, e
                );
                return Err(format!("PDF extraction failed: {}", e).into());
            }
        }
    } else {
        debug!("Processing as text file: {}", file_path);
        // Convert bytes to a UTF‑8 String
        match String::from_utf8(bytes) {
            Ok(text) => {
                debug!("Successfully converted to UTF-8, length: {}", text.len());
                text
            }
            Err(e) => {
                error!(
                    "Failed to convert S3 response to UTF-8 for bucket {} key {}: {}",
                    bucket_name, file_path, e
                );
                // If it's not valid UTF-8, return as base64 or error
                return Err("File content is not valid UTF-8 text".into());
            }
        }
    };

    info!(
        "Successfully retrieved file from bucket: {}, content length: {}",
        file_path,
        content.len()
    );
    Ok(content)
}
