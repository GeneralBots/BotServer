use super::*;

pub(super) fn public_link_url(token: &str) -> String {
    format!("/api/files/public/{token}")
}
pub(super) fn format_link_time(dt: Option<chrono::DateTime<chrono::Utc>>) -> Option<String> {
    dt.map(|d| d.to_rfc3339())
}
/// `POST /api/files/share-link` — create a revocable public link for a file.
pub async fn create_public_link(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreatePublicLinkBody>,
) -> Result<Json<CreatePublicLinkResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let path = normalize_path(&req.path);
    if path.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "path is required"));
    }
    if path.ends_with('/') {
        return Err(err(StatusCode::BAD_REQUEST, "Public links are only available for files"));
    }
    let key = format!("{prefix}{path}");

    // The file must exist and belong to the caller before a link is minted.
    drive
        .get_object(&bucket, &key)
        .await
        .map_err(|_| err(StatusCode::NOT_FOUND, "File not found"))?;

    let expires_at = match req.expires_at.as_deref() {
        None | Some("") => None,
        Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
            Err(_) => return Err(err(StatusCode::BAD_REQUEST, "expires_at must be RFC3339")),
        },
    };

    let token = uuid::Uuid::new_v4().simple().to_string();
    let mut conn = state
        .conn
        .get()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db pool: {e}")))?;

    let inserted = diesel::sql_query(
        "INSERT INTO drive_public_links (owner_id, bucket, path, key, scope, share_token, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind::<Text, _>(&uid)
    .bind::<Text, _>(&bucket)
    .bind::<Text, _>(&path)
    .bind::<Text, _>(&key)
    .bind::<Text, _>(scope_label(&scope))
    .bind::<Text, _>(&token)
    .bind::<Nullable<Timestamptz>, _>(expires_at)
    .execute(&mut conn)
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to create link: {e}")))?;

    if inserted == 0 {
        return Err(err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create link"));
    }

    info!("Public link created for {path} in {bucket} by {uid}");
    Ok(Json(CreatePublicLinkResponse {
        token: token.clone(),
        url: public_link_url(&token),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: format_link_time(expires_at),
    }))
}
pub(super) fn scope_label(scope: &FileScope) -> &'static str {
    match scope {
        FileScope::User => "user",
        FileScope::Bot => "bot",
    }
}
/// `GET /api/files/share-links` — list the caller's public links (optionally
/// for one file, so the UI can show "copy" instead of "create").
pub async fn list_public_links(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListPublicLinksParams>,
) -> Result<Json<Vec<PublicLinkItem>>, (StatusCode, Json<serde_json::Value>)> {
    let uid = effective_uid(&state, &user, params.user_id.as_deref());
    let mut conn = state
        .conn
        .get()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db pool: {e}")))?;

    let path_filter = params.path.as_deref().unwrap_or("").trim();
    let rows = if path_filter.is_empty() {
        diesel::sql_query(
            "SELECT bucket, key, path, share_token, created_at, expires_at, revoked \
             FROM drive_public_links WHERE owner_id = $1 ORDER BY created_at DESC",
        )
        .bind::<Text, _>(&uid)
        .load::<PublicLinkRow>(&mut conn)
        .unwrap_or_default()
    } else {
        diesel::sql_query(
            "SELECT bucket, key, path, share_token, created_at, expires_at, revoked \
             FROM drive_public_links WHERE owner_id = $1 AND path = $2 ORDER BY created_at DESC",
        )
        .bind::<Text, _>(&uid)
        .bind::<Text, _>(path_filter)
        .load::<PublicLinkRow>(&mut conn)
        .unwrap_or_default()
    };

    let items: Vec<PublicLinkItem> = rows
        .into_iter()
        .map(|r| PublicLinkItem {
            url: public_link_url(&r.share_token),
            token: r.share_token,
            bucket: r.bucket,
            path: r.path,
            created_at: format_link_time(r.created_at).unwrap_or_default(),
            expires_at: format_link_time(r.expires_at),
            revoked: r.revoked,
        })
        .collect();

    Ok(Json(items))
}
/// `POST /api/files/revoke-link` — revoke by token, or by file (bucket+path)
/// for the caller. Revoked links keep their row (audit trail) but stop
/// resolving immediately.
pub async fn revoke_public_link(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<RevokePublicLinkBody>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let mut conn = state
        .conn
        .get()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db pool: {e}")))?;

    let changed = if let Some(token) = req.token.as_deref().filter(|t| !t.is_empty()) {
        diesel::sql_query(
            "UPDATE drive_public_links SET revoked = TRUE \
             WHERE share_token = $1 AND owner_id = $2",
        )
        .bind::<Text, _>(token)
        .bind::<Text, _>(&uid)
        .execute(&mut conn)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to revoke link: {e}")))?
    } else if let Some(path) = req.path.as_deref().filter(|p| !p.is_empty()) {
        let bucket = req
            .bucket
            .clone()
            .unwrap_or_else(|| state.bucket_name.clone());
        diesel::sql_query(
            "UPDATE drive_public_links SET revoked = TRUE \
             WHERE owner_id = $1 AND bucket = $2 AND path = $3",
        )
        .bind::<Text, _>(&uid)
        .bind::<Text, _>(&bucket)
        .bind::<Text, _>(path)
        .execute(&mut conn)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to revoke link: {e}")))?
    } else {
        return Err(err(StatusCode::BAD_REQUEST, "token or path is required"));
    };

    if changed == 0 {
        return Err(err(StatusCode::NOT_FOUND, "No matching public link"));
    }

    info!("Public link revoked by {uid}");
    Ok(Json(SuccessResponse { success: true }))
}
/// `GET /api/files/public/{token}` — anonymous download of a shared file.
/// Registered on the anonymous-path allowlist; safe because access is
/// token-only and the token is captured at creation time.
pub async fn public_link_download(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let token = token.trim().to_lowercase();
    if token.is_empty() || token.len() > 64 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(err(StatusCode::NOT_FOUND, "Link not found"));
    }

    let mut conn = state
        .conn
        .get()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db pool: {e}")))?;

    let rows = diesel::sql_query(
        "SELECT bucket, key, path, share_token, created_at, expires_at, revoked \
         FROM drive_public_links WHERE share_token = $1",
    )
    .bind::<Text, _>(&token)
    .load::<PublicLinkRow>(&mut conn)
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db error: {e}")))?;

    let row = match rows.into_iter().next() {
        Some(r) => r,
        None => return Err(err(StatusCode::NOT_FOUND, "Link not found")),
    };

    // Revoked and expired links are indistinguishable from unknown links.
    if row.revoked {
        return Err(err(StatusCode::NOT_FOUND, "Link not found"));
    }
    if let Some(expires) = row.expires_at {
        if expires < chrono::Utc::now() {
            return Err(err(StatusCode::NOT_FOUND, "Link not found"));
        }
    }

    let drive = get_drive(&state)?;
    let data = drive
        .get_object(&row.bucket, &row.key)
        .await
        .map_err(|_| err(StatusCode::NOT_FOUND, "Link not found"))?;

    let file_name = row.path.rsplit('/').next().unwrap_or("download").to_string();
    let mime = guess_mime(&file_name);

    use axum::response::IntoResponse;
    let headers = [
        (axum::http::header::CONTENT_TYPE, mime.to_string()),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file_name),
        ),
        (axum::http::header::CONTENT_LENGTH, data.len().to_string()),
        (axum::http::header::CACHE_CONTROL, "private, no-store".to_string()),
        (axum::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
    ];
    Ok((headers, data).into_response())
}
