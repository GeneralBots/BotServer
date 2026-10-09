use super::*;

pub async fn list_trash(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<TrashQueryParams>,
) -> Result<Json<Vec<TrashItem>>, (StatusCode, Json<serde_json::Value>)> {
    let uid = effective_uid(&state, &user, params.user_id.as_deref());
    if let Ok(mut conn) = state.conn.get() {
        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            user_id: String,
            #[diesel(sql_type = Text)]
            bucket: String,
            #[diesel(sql_type = Text)]
            path: String,
            #[diesel(sql_type = Text)]
            original_path: String,
            #[diesel(sql_type = Bool)]
            is_dir: bool,
            #[diesel(sql_type = BigInt)]
            size: i64,
            #[diesel(sql_type = Text)]
            deleted_at: String,
            #[diesel(sql_type = Text)]
            expires_at: String,
        }
        let rows = diesel::sql_query(
            "SELECT id::text, user_id, bucket, path, original_path, is_dir, size::bigint, deleted_at::text, expires_at::text FROM drive_trash WHERE user_id = $1 ORDER BY deleted_at DESC"
        )
        .bind::<Text, _>(uid)
        .load::<Row>(&mut conn)
        .unwrap_or_default();

        let items: Vec<TrashItem> = rows.into_iter().map(|r| TrashItem {
            id: r.id,
            user_id: r.user_id,
            bucket: r.bucket,
            path: r.path,
            original_path: r.original_path,
            is_dir: r.is_dir,
            size: r.size as u64,
            deleted_at: r.deleted_at,
            expires_at: r.expires_at,
        }).collect();
        return Ok(Json(items));
    }
    Ok(Json(vec![]))
}
pub async fn trash_file(
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

    let data = match drive.get_object(&bucket, &key).await {
        Ok(d) => d,
        Err(e) => return Err(err(StatusCode::NOT_FOUND, &format!("File not found: {e}"))),
    };

    let trash_key = format!(".trash/{}/{}", uid, normalize_path(&req.path));
    let data_len = data.len() as i64;
    drive
        .put_object(&bucket, &trash_key, data, None)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Trash move failed: {e}")))?;

    let _ = drive.delete_object(&bucket, &key).await;

    if let Ok(mut conn) = state.conn.get() {
        let _ = diesel::sql_query(
            "INSERT INTO drive_trash (user_id, bucket, path, original_path, size) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind::<Text, _>(uid)
        .bind::<Text, _>(&bucket)
        .bind::<Text, _>(&trash_key)
        .bind::<Text, _>(&key)
        .bind::<BigInt, _>(data_len)
        .execute(&mut conn);
    }

    info!("File moved to trash: {key} -> {trash_key}");
    Ok(Json(SuccessResponse { success: true }))
}
pub async fn restore_trash(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<RestoreTrashBody>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let uid = effective_uid(&state, &user, req.user_id.as_deref());

    if let Ok(mut conn) = state.conn.get() {
        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = Text)]
            bucket: String,
            #[diesel(sql_type = Text)]
            path: String,
            #[diesel(sql_type = Text)]
            original_path: String,
        }
        let rows = diesel::sql_query(
            "SELECT bucket, path, original_path FROM drive_trash WHERE id::text = $1 AND user_id = $2"
        )
        .bind::<Text, _>(&req.id)
        .bind::<Text, _>(&uid)
        .load::<Row>(&mut conn)
        .unwrap_or_default();

        if let Some(row) = rows.into_iter().next() {
            if let Ok(data) = drive.get_object(&row.bucket, &row.path).await {
                let _ = drive.put_object(&row.bucket, &row.original_path, data, None).await;
                let _ = drive.delete_object(&row.bucket, &row.path).await;
            }
            let _ = diesel::sql_query("DELETE FROM drive_trash WHERE id::text = $1 AND user_id = $2")
                .bind::<Text, _>(&req.id)
                .bind::<Text, _>(&uid)
                .execute(&mut conn);
        }
    }

    Ok(Json(SuccessResponse { success: true }))
}
pub async fn empty_trash(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<EmptyTrashBody>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let uid = effective_uid(&state, &user, req.user_id.as_deref());

    if let Ok(mut conn) = state.conn.get() {
        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = Text)]
            bucket: String,
            #[diesel(sql_type = Text)]
            path: String,
        }
        let rows = diesel::sql_query(
            "SELECT bucket, path FROM drive_trash WHERE user_id = $1"
        )
        .bind::<Text, _>(&uid)
        .load::<Row>(&mut conn)
        .unwrap_or_default();

        for row in &rows {
            let _ = drive.delete_object(&row.bucket, &row.path).await;
        }

        let _ = diesel::sql_query("DELETE FROM drive_trash WHERE user_id = $1")
            .bind::<Text, _>(&uid)
            .execute(&mut conn);
    }

    Ok(Json(SuccessResponse { success: true }))
}
pub async fn upload_file_binary(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;

    let data = body.into_bytes();
    let bucket = state.bucket_name.clone();
    let key = format!("uploads/{}", uuid::Uuid::new_v4());

    drive
        .put_object(&bucket, &key, data, None)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Upload failed: {e}")))?;

    Ok(Json(SuccessResponse { success: true }))
}
