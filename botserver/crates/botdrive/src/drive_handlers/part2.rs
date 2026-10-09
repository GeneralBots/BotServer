use super::*;

pub async fn list_files(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListFilesParams>,
) -> Result<Json<Vec<FileListItem>>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = params.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, params.user_id.as_deref());
    let bucket = resolve_bucket(&state, params.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let sub_path = normalize_path(params.path.as_deref().unwrap_or(""));
    let full_prefix = if sub_path.is_empty() {
        prefix
    } else {
        format!("{prefix}{sub_path}/")
    };

    // One LIST carries key + size, so the second, identical listing was pure
    // waste: `list_objects` is just `list_objects_with_metadata` mapped to
    // keys, meaning every folder navigation hit MinIO twice for the same
    // prefix. `create_bucket_if_not_exists` moved to the error path too — the
    // HEAD it performs on every listing only matters when the bucket is
    // actually missing (a user's personal bucket), and paying for it on every
    // request starved the CPU the handler needs.
    let objects = match drive.list_objects_with_metadata(&bucket, Some(&full_prefix)).await {
        Ok(objects) => objects,
        Err(list_err) => {
            // Ensures a personal bucket that has never been written to still
            // renders as an empty folder instead of erroring.
            warn!("drive: listing '{full_prefix}' failed ({list_err}); ensuring bucket '{bucket}' exists");
            let _ = drive.create_bucket_if_not_exists(&bucket).await;
            drive.list_objects_with_metadata(&bucket, Some(&full_prefix))
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to list objects: {e}")))?
        }
    };

    let keys: Vec<String> = objects.iter().map(|o| o.key.clone()).collect();
    let meta_map: HashMap<String, u64> = objects.into_iter().map(|o| (o.key, o.size)).collect();

    let items = build_file_list_items(&full_prefix, &keys, &meta_map);
    Ok(Json(items))
}
pub async fn upload_file_to_drive(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<WriteFileBody>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);

    // Accept both raw text (designer/editor send the file content verbatim)
    // and base64-encoded payloads (drive uploads). Detect: a strict base64
    // decode that round-trips cleanly (ignoring padding) is treated as
    // encoded; otherwise the content is used as-is so text files never fail
    // on invalid base64.
    let data = match base64::engine::general_purpose::STANDARD.decode(&req.content) {
        Ok(decoded) if !decoded.is_empty() => {
            let re_encoded = base64::engine::general_purpose::STANDARD.encode(&decoded);
            let normalized_input = req.content.trim_end_matches('=');
            let normalized_encoded = re_encoded.trim_end_matches('=');
            if normalized_encoded == normalized_input {
                decoded
            } else {
                req.content.as_bytes().to_vec()
            }
        }
        _ => req.content.as_bytes().to_vec(),
    };

    let key = format!("{prefix}{}", normalize_path(&req.path));
    drive
        .put_object(&bucket, &key, data, None)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Upload failed: {e}")))?;

    info!("File uploaded: {key} to bucket {bucket}");
    Ok(Json(SuccessResponse { success: true }))
}
/// `POST /api/files/read` — returns a file's content as raw text (`{content}`)
/// for the designer/editor apps. Mirrors `download_file` but returns plain
/// UTF-8 text instead of base64 so BASIC sources render directly.
pub async fn read_file(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<DownloadFileBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let key = format!("{prefix}{}", normalize_path(&req.path));

    let data = drive
        .get_object(&bucket, &key)
        .await
        .map_err(|e| err(StatusCode::NOT_FOUND, &format!("File not found: {e}")))?;

    let content = String::from_utf8_lossy(&data).to_string();
    Ok(Json(serde_json::json!({ "content": content })))
}
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<DownloadFileBody>,
) -> Result<Json<DownloadFileResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let key = format!("{prefix}{}", normalize_path(&req.path));

    let data = drive
        .get_object(&bucket, &key)
        .await
        .map_err(|e| err(StatusCode::NOT_FOUND, &format!("File not found: {e}")))?;

    let content = base64::engine::general_purpose::STANDARD.encode(&data);
    let file_name = req.path.rsplit('/').next().unwrap_or("download").to_string();

    Ok(Json(DownloadFileResponse { content, file_name }))
}
pub async fn download_file_binary(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<DownloadFileBody>,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let key = format!("{prefix}{}", normalize_path(&req.path));

    let data = drive
        .get_object(&bucket, &key)
        .await
        .map_err(|e| err(StatusCode::NOT_FOUND, &format!("File not found: {e}")))?;

    let file_name = req.path.rsplit('/').next().unwrap_or("download").to_string();
    let mime = guess_mime(&file_name);

    use axum::response::IntoResponse;
    let headers = [
        (axum::http::header::CONTENT_TYPE, mime.to_string()),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file_name),
        ),
        (axum::http::header::CONTENT_LENGTH, data.len().to_string()),
    ];
    Ok((headers, data).into_response())
}
/// #1306 — raw file bytes served INLINE (no Content-Disposition attachment),
/// so apps like the Photo Editor can load Drive images into a canvas with
/// `fetch` + `createObjectURL` instead of base64 round-trips.
pub async fn download_file_inline(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<DownloadFileBody>,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let key = format!("{prefix}{}", normalize_path(&req.path));

    let data = drive
        .get_object(&bucket, &key)
        .await
        .map_err(|e| err(StatusCode::NOT_FOUND, &format!("File not found: {e}")))?;

    let file_name = req.path.rsplit('/').next().unwrap_or("download").to_string();
    let mime = guess_mime(&file_name);

    use axum::response::IntoResponse;
    let headers = [
        (axum::http::header::CONTENT_TYPE, mime.to_string()),
        (axum::http::header::CONTENT_LENGTH, data.len().to_string()),
        (axum::http::header::CACHE_CONTROL, "no-store".to_string()),
    ];
    Ok((headers, data).into_response())
}
pub(super) fn guess_mime(file_name: &str) -> &'static str {
    let ext = file_name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "txt" | "log" | "md" => "text/plain; charset=utf-8",
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<DeleteFileBody>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let key = format!("{prefix}{}", normalize_path(&req.path));

    drive
        .delete_object(&bucket, &key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Delete failed: {e}")))?;

    info!("File deleted: {key} from bucket {bucket}");
    Ok(Json(SuccessResponse { success: true }))
}
pub async fn create_folder(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreateFolderBody>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let parent = normalize_path(&req.path);
    let folder_name = normalize_path(&req.name);
    let key = if parent.is_empty() {
        format!("{prefix}{folder_name}/")
    } else {
        format!("{prefix}{parent}/{folder_name}/")
    };

    drive
        .put_object(&bucket, &key, vec![], None)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Create folder failed: {e}")))?;

    info!("Folder created: {key} in bucket {bucket}");
    Ok(Json(SuccessResponse { success: true }))
}
pub async fn copy_file(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CopyFileBody>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let prefix = resolve_scope_prefix(&scope, &uid);

    let src_bucket = req.source_bucket.as_deref().unwrap_or(&state.bucket_name);
    let dest_bucket = req.dest_bucket.as_deref().unwrap_or(&state.bucket_name);
    let from_key = format!("{prefix}{}", normalize_path(&req.source_path));
    let to_key = format!("{prefix}{}", normalize_path(&req.dest_path));

    let data = drive
        .get_object(src_bucket, &from_key)
        .await
        .map_err(|e| err(StatusCode::NOT_FOUND, &format!("Source file not found: {e}")))?;

    drive
        .put_object(dest_bucket, &to_key, data, None)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Copy failed: {e}")))?;

    info!("Copied {from_key} -> {to_key} in bucket {dest_bucket}");
    Ok(Json(SuccessResponse { success: true }))
}
