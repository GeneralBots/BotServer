use super::*;

#[cfg(not(feature = "vibe"))]
pub async fn serve_app_index(
    State(state): State<Arc<AppState>>,
    Path(params): Path<AppPath>,
    original_uri: axum::extract::OriginalUri,
) -> impl IntoResponse {
    // Redirect to trailing slash so relative paths resolve correctly
    // /apps/calc-pro -> /apps/calc-pro/
    let path = original_uri.path();
    if !path.ends_with('/') {
        return axum::response::Redirect::permanent(&format!("{}/", path)).into_response();
    }
    serve_app_file_internal(&state, &params.app_name, "index.html").await
}
#[cfg(not(feature = "vibe"))]
pub async fn serve_app_file(
    State(state): State<Arc<AppState>>,
    Path(params): Path<AppFilePath>,
) -> impl IntoResponse {
    serve_app_file_internal(&state, &params.app_name, &params.file_path).await
}
/// Sanitize app name - only alphanumeric, underscore, hyphen allowed
#[cfg(not(feature = "vibe"))]
pub(super) fn sanitize_app_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>()
}
/// Sanitize file path - preserve directory structure but remove dangerous characters
pub(super) fn sanitize_file_path(path: &str) -> String {
    path.split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".." && *segment != ".")
        .map(|segment| {
            segment
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}
#[cfg(not(feature = "vibe"))]
pub(super) async fn serve_app_file_internal(state: &AppState, app_name: &str, file_path: &str) -> Response {
    let sanitized_app_name = sanitize_app_name(app_name);
    let sanitized_file_path = sanitize_file_path(file_path);

    if sanitized_app_name.is_empty() || sanitized_file_path.is_empty() {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    // Get bot name from bucket_name config (default to "default")
    let bot_name = state.bucket_name.trim_end_matches(".gbai").to_string();
    let sanitized_bot_name = bot_name.to_lowercase().replace([' ', '_'], "-");

    // MinIO bucket and path: botname.gbai / botname.gbapp/appname/file
    let bucket = format!("{}.gbai", sanitized_bot_name);
    let key = format!(
        "{}.gbapp/{}/{}",
        sanitized_bot_name, sanitized_app_name, sanitized_file_path
    );

    info!(
        "Serving app file from MinIO: bucket={}, key={}",
        bucket, key
    );

    // Try to serve from MinIO
    #[cfg(feature = "drive")]
    if let Some(ref drive) = state.drive {
        match drive.get_object(&bucket, &key).await {
                    Ok(content) => {
                        let content_type = get_content_type(&sanitized_file_path);

                        let final_content = if content_type.starts_with("text/html") {
                            let html = String::from_utf8_lossy(&content);
                            let rewritten = rewrite_cdn_urls(&html);
                            rewritten.into_bytes()
        } else {
            content.to_vec()
        };

                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::CACHE_CONTROL, "public, max-age=3600")
                    .body(Body::from(final_content))
                    .unwrap_or_else(|_| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to build response",
                        )
                            .into_response()
                    });
            }
            Err(e) => {
                log::error!("Failed to get object: {}", e);
            }
        }
    }

// Fallback to filesystem if MinIO fails
    let site_path = state
        .config
        .as_ref()
        .map(|c| c.site_path.clone())
        .unwrap_or_else(|| format!("{}/sites", get_stack_path()));

    let full_path = format!(
        "{site_path}/{org_id}.gborg/{sanitized_bot_name}.gbai/{sanitized_bot_name}.gbapp/{sanitized_app_name}/{sanitized_file_path}",
        org_id = botcore::shared::utils::current_org_id()
    );

    trace!("Fallback: serving app file from filesystem: {full_path}");

    let path = std::path::Path::new(&full_path);
    if !path.exists() {
        warn!("App file not found: {full_path}");
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    let content_type = get_content_type(&sanitized_file_path);

    match std::fs::read(&full_path) {
        Ok(contents) => {
            // For HTML files, rewrite CDN URLs to local paths
            let final_content = if content_type.starts_with("text/html") {
                let html = String::from_utf8_lossy(&contents);
                rewrite_cdn_urls(&html).into_bytes()
            } else {
                contents
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(Body::from(final_content))
                .unwrap_or_else(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to build response",
                    )
                        .into_response()
                })
        }
        Err(e) => {
            log::error!("Failed to read file {full_path}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response()
        }
    }
}
#[cfg(not(feature = "vibe"))]
pub async fn list_all_apps(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let site_path = state
        .config
        .as_ref()
        .map(|c| c.site_path.clone())
        .unwrap_or_else(|| format!("{}/sites", get_stack_path()));

    let mut apps = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&site_path) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with('.') || name.to_lowercase().ends_with(".gbai") {
                        continue;
                    }

                    let app_path = entry.path();
                    let has_index = app_path.join("index.html").exists();

                    if has_index {
                        apps.push(serde_json::json!({
                            "name": name,
                            "url": format!("/apps/{}", name),
                            "has_index": true
                        }));
                    }
                }
            }
        }
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "apps": apps,
            "count": apps.len()
        })),
    )
        .into_response()
}
