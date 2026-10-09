use super::*;

pub async fn serve_suite_js_file(
    State(state): State<Arc<AppState>>,
    Path(params): Path<SuiteJsFilePath>,
) -> Response {
    let file_path = sanitize_file_path(&params.file_path);

    if file_path.is_empty() {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    if file_path.starts_with("vendor/") || file_path.starts_with("vendor\\") {
        return serve_vendor_file(
            State(state),
            Path(VendorFilePath {
                file_path: file_path
                    .strip_prefix("vendor/")
                    .unwrap_or(&file_path)
                    .to_string(),
            }),
        )
        .await;
    }

    if !file_path.ends_with(".js") {
        return (StatusCode::BAD_REQUEST, "Only JS files allowed").into_response();
    }

    let content_type = get_content_type(&file_path);

    let ui_path = std::env::var("BOTUI_PATH").unwrap_or_else(|_| {
        if std::path::Path::new("./botui/ui/suite").exists() {
            "./botui/ui/suite".to_string()
        } else if std::path::Path::new("../botui/ui/suite").exists() {
            "../botui/ui/suite".to_string()
        } else {
            "./botui/ui/suite".to_string()
        }
    });
    let local_path = format!("{}/js/{}", ui_path, file_path);

    match tokio::fs::read(&local_path).await {
        Ok(content) => {
            trace!("Serving suite JS file from: {}", local_path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(Body::from(content))
                .unwrap_or_else(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to build response",
                    )
                        .into_response()
                })
        }
        Err(e) => {
            warn!("Suite JS file not found: {} - {}", local_path, e);
            (StatusCode::NOT_FOUND, "JS file not found").into_response()
        }
    }
}
pub async fn serve_vendor_file(
    State(state): State<Arc<AppState>>,
    Path(params): Path<VendorFilePath>,
) -> Response {
    let file_path = sanitize_file_path(&params.file_path);

    if file_path.is_empty() {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    let content_type = get_content_type(&file_path);

    let local_paths = [
        format!("./botui/ui/suite/js/vendor/{}", file_path),
        format!("../botui/ui/suite/js/vendor/{}", file_path),
        format!("{}/static/js/vendor/{}", get_stack_path(), file_path),
    ];

    for local_path in &local_paths {
        if let Ok(content) = tokio::fs::read(local_path).await {
            trace!("Serving vendor file from local path: {}", local_path);
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=86400")
                .body(Body::from(content))
                .unwrap_or_else(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to build response",
                    )
                        .into_response()
                });
        }
    }

    let bot_name = state.bucket_name.trim_end_matches(".gbai").to_string();
    let sanitized_bot_name = bot_name.to_lowercase().replace([' ', '_'], "-");

    let bucket = format!("{}.gbai", sanitized_bot_name);
    let key = format!("{}.gblib/vendor/{}", sanitized_bot_name, file_path);

    trace!(
        "Trying MinIO for vendor file: bucket={}, key={}",
        bucket,
        key
    );

#[cfg(feature = "drive")]
    if let Some(ref drive) = state.drive {
        match drive.get_object(&bucket, &key).await {
                    Ok(content) => {
                        return Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CACHE_CONTROL, "public, max-age=86400")
                        .body(Body::from(content))
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

(StatusCode::NOT_FOUND, "Vendor file not found").into_response()
}
#[cfg(not(feature = "vibe"))]
pub(super) fn rewrite_cdn_urls(html: &str) -> String {
    html
        // HTMX from various CDNs
        .replace(
            "https://unpkg.com/htmx.org@1.9.10",
            "/js/vendor/htmx.min.js",
        )
        .replace(
            "https://unpkg.com/htmx.org@1.9.10/dist/htmx.min.js",
            "/js/vendor/htmx.min.js",
        )
        .replace(
            "https://unpkg.com/htmx.org@1.9.11",
            "/js/vendor/htmx.min.js",
        )
        .replace(
            "https://unpkg.com/htmx.org@1.9.11/dist/htmx.min.js",
            "/js/vendor/htmx.min.js",
        )
        .replace(
            "https://unpkg.com/htmx.org@1.9.12",
            "/js/vendor/htmx.min.js",
        )
        .replace(
            "https://unpkg.com/htmx.org@1.9.12/dist/htmx.min.js",
            "/js/vendor/htmx.min.js",
        )
        .replace("https://unpkg.com/htmx.org", "/js/vendor/htmx.min.js")
        .replace(
            "https://cdn.jsdelivr.net/npm/htmx.org",
            "/js/vendor/htmx.min.js",
        )
        .replace(
            "https://cdnjs.cloudflare.com/ajax/libs/htmx/1.9.10/htmx.min.js",
            "/js/vendor/htmx.min.js",
        )
        .replace(
            "https://cdnjs.cloudflare.com/ajax/libs/htmx/1.9.11/htmx.min.js",
            "/js/vendor/htmx.min.js",
        )
        .replace(
            "https://cdnjs.cloudflare.com/ajax/libs/htmx/1.9.12/htmx.min.js",
            "/js/vendor/htmx.min.js",
        )
}
pub fn configure_app_server_routes() -> Router<Arc<AppState>> {
    let router = Router::new()
    // Serve shared vendor files from MinIO: /js/vendor/*
    .route("/js/vendor/*file_path", get(serve_vendor_file))
    // Serve suite JS files (i18n.js, theme-manager.js, etc.)
    .route("/js/*file_path", get(serve_suite_js_file));

    #[cfg(not(feature = "vibe"))]
    let router = router
        // Legacy app serving (removed under vibe feature — apps/sites served by Caddy/Incus)
        .route("/apps/:app_name", get(serve_app_index))
        .route("/apps/:app_name/", get(serve_app_index))
        .route("/apps/:app_name/*file_path", get(serve_app_file))
        .route("/apps", get(list_all_apps));

    router
}
