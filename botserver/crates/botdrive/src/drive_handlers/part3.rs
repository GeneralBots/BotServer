use super::*;

pub async fn move_file(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<MoveFileBody>,
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
        .map_err(|e| err(StatusCode::NOT_FOUND, &format!("Source not found: {e}")))?;

    drive
        .put_object(dest_bucket, &to_key, data, None)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Move (write) failed: {e}")))?;

    let _ = drive
        .delete_object(src_bucket, &from_key)
        .await
        .map_err(|e| {
            warn!("Move: failed to delete source {from_key}: {e}");
        });

    info!("Moved {from_key} -> {to_key}");
    Ok(Json(SuccessResponse { success: true }))
}
pub async fn search_files(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<SearchQueryParams>,
) -> Result<Json<Vec<FileListItem>>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = params.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, params.user_id.as_deref());
    let bucket = resolve_bucket(&state, params.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);
    let query = params.query.unwrap_or_default();
    let query_lower = query.to_lowercase();

    let keys = drive
        .list_objects(&bucket, Some(&prefix))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Search failed: {e}")))?;

    let mut meta_map: HashMap<String, u64> = HashMap::new();
    if let Ok(objects) = drive.list_objects_with_metadata(&bucket, Some(&prefix)).await {
        for obj in objects {
            meta_map.insert(obj.key, obj.size);
        }
    }

    let all_items = build_file_list_items(&prefix, &keys, &meta_map);
    let filtered: Vec<FileListItem> = all_items
        .into_iter()
        .filter(|item| item.name.to_lowercase().contains(&query_lower))
        .collect();

    Ok(Json(filtered))
}
pub async fn recent_files(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<RecentQueryParams>,
) -> Result<Json<Vec<FileListItem>>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;
    let scope = params.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, params.user_id.as_deref());
    let bucket = resolve_bucket(&state, params.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;
    let prefix = resolve_scope_prefix(&scope, &uid);

    if let Ok(mut conn) = state.conn.get() {
        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Text)]
            file_path: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            file_type: String,
            #[diesel(sql_type = Nullable<BigInt>)]
            file_size: Option<i64>,
            #[diesel(sql_type = Nullable<Timestamptz>)]
            last_modified: Option<chrono::DateTime<chrono::Utc>>,
        }
        let rows = diesel::sql_query(
            "SELECT file_path, file_type, file_size, last_modified FROM drive_files WHERE file_path LIKE $1 || '/%' AND indexed = true ORDER BY last_modified DESC NULLS LAST LIMIT 50"
        )
        .bind::<Text, _>(&bucket)
        .load::<Row>(&mut conn);

        if let Ok(values) = rows {
            let items: Vec<FileListItem> = values.into_iter().map(|r| {
                let file_path = &r.file_path;
                let relative = if file_path.starts_with(&prefix) {
                    file_path[prefix.len()..].to_string()
                } else {
                    file_path.to_string()
                };
                FileListItem {
                    is_dir: false,
                    name: relative.rsplit('/').next().unwrap_or(relative.as_str()).to_string(),
                    path: relative,
                    size: r.file_size.unwrap_or(0) as u64,
                    modified: r.last_modified.map(|d| d.to_rfc3339()),
                    is_kb: r.file_type == "kb",
                    is_public: false,
                }
            }).collect();
            return Ok(Json(items));
        }
    }

    let keys = drive
        .list_objects(&bucket, Some(&prefix))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to list recent: {e}")))?;

    let mut meta_map: HashMap<String, u64> = HashMap::new();
    if let Ok(objects) = drive.list_objects_with_metadata(&bucket, Some(&prefix)).await {
        for obj in objects {
            meta_map.insert(obj.key, obj.size);
        }
    }

    let mut fallback_items = build_file_list_items(&prefix, &keys, &meta_map);
    fallback_items.retain(|item| !item.is_dir);
    fallback_items.sort_by(|a, b| b.name.cmp(&a.name));
    fallback_items.truncate(50);
    Ok(Json(fallback_items))
}
pub async fn list_buckets(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListBucketsParams>,
) -> Result<Json<Vec<BucketListItem>>, (StatusCode, Json<serde_json::Value>)> {
    let drive = get_drive(&state)?;

    let bucket_names = drive
        .list_all_buckets()
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to list buckets: {e}")))?;

    // Tenant isolation (#1401): non-admins only discover buckets they are
    // entitled to — their personal user- bucket, the instance default, and
    // one {slug}.gborg per org they belong to. Previously the filter let
    // every non-.gbai bucket through, so ANY signed-in user could see and
    // then list OTHER orgs' {slug}.gborg buckets (cross-tenant leak).
    // Bypass is SUPER-admin only: the global RBAC `admin` group also holds
    // plain org admins, who must stay confined to their own tenant.
    let platform_admin = is_platform_admin(&state, &user);
    let owned: Vec<String> = if platform_admin {
        Vec::new()
    } else {
        allowed_buckets_for(&state, &user)
    };

    // Resolve the bot filter once, before the loop: an org bot lives in
    // `{slug}.gborg` and a standalone bot in `{bot}.gbai`, so guessing both
    // shapes from the name would also match a bucket that does not belong to
    // this bot. `None` means no filter, or an unresolvable bot (which matches
    // nothing rather than silently exposing every bucket).
    let bot_bucket: Option<String> = params.bot.as_deref().map(|bot| {
        match state.conn.get() {
            Ok(mut conn) => {
                Some(botbasic_core::utils::bot_drive_location_for_name(&mut conn, bot).bucket)
            }
            Err(e) => {
                warn!("drive: bucket filter lookup failed for bot '{bot}': {e}");
                None
            }
        }
    }).flatten();

    let items: Vec<BucketListItem> = bucket_names
        .into_iter()
        .filter(|name| {
            if !platform_admin {
                // Personal bucket, instance default, or caller's own org
                // workspace bucket only. Other tenants are invisible.
                return owned.iter().any(|a| a == name);
            }
            match (params.bot.as_deref(), &bot_bucket) {
                (Some(_), Some(bucket)) => name == bucket,
                (Some(_), None) => false,
                (None, _) => true,
            }
        })
        .map(|name| BucketListItem {
            is_gbai: name.ends_with(".gbai"),
            is_gborg: name.ends_with(".gborg"),
            name,
        })
        .collect();

    Ok(Json(items))
}
pub async fn create_bot(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreateBotRequest>,
) -> Result<Json<CreateBotResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Tenant isolation (#1387): only admins may provision bots, and an org
    // admin may only provision inside its own tenant (name-scoped below).
    if !is_admin_user(&user) {
        return Err(err(StatusCode::FORBIDDEN, "Administrator access required"));
    }
    let name = req.name.trim().to_lowercase();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(err(StatusCode::BAD_REQUEST, "Invalid bot name. Use lowercase letters, numbers, and hyphens (3-50 chars)."));
    }
    if name.len() < 3 || name.len() > 50 {
        return Err(err(StatusCode::BAD_REQUEST, "Bot name must be between 3 and 50 characters."));
    }

    // Resolve the target bucket from the bots table: an org-hosted bot is
    // seeded inside the org workspace bucket (`{tenant}.gborg`) under its
    // branch's `{branch}.gbai/{name}.gb…` tree, never in its own `{name}.gbai`
    // bucket — that standalone bucket would be dead weight nothing reads.
    // Falls back to the standalone layout when the bot has no row or no org yet.
    let location = match state.conn.get() {
        Ok(mut conn) => botbasic_core::utils::bot_drive_location_for_name(&mut conn, &name),
        Err(e) => {
            return Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("Database unavailable: {e}"),
            ))
        }
    };
    let bucket_name = location.bucket.clone();
    let drive = get_drive(&state)?;

    // Check if already exists — a seeded org bot shares its bucket with its
    // siblings, so the existence test is on the bot's own key prefix, not on
    // the bucket name.
    let seeded_marker = format!("{}{name}.gbdrive/.keep", location.bot_prefix);
    let existing = drive.list_objects(&bucket_name, Some(&seeded_marker)).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to inspect bucket: {e}")))?;
    if !existing.is_empty() {
        return Err(err(StatusCode::CONFLICT, &format!("Bot '{}' already exists", name)));
    }

    // Create bucket (a no-op for the org workspace, which already exists)
    drive.create_bucket_if_not_exists(&bucket_name).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to create bucket: {e}")))?;

    // Seed default folder structure with .keep marker files
    let seed_paths = [
        format!("{name}.gbdialog/.keep"),
        format!("{name}.gbkb/.keep"),
        format!("{name}.gbdrive/.keep"),
        format!("{name}.gbot/.keep"),
        format!("{name}.gbot/config.csv"),
    ];
    for path in &seed_paths {
        let content = if path.ends_with("config.csv") {
            "system-prompt,\nhistory-limit,6\n".as_bytes().to_vec()
        } else {
            Vec::new()
        };
        let key = format!("{}{path}", location.bot_prefix);
        drive.put_object(&bucket_name, &key, content, Some("text/plain")).await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to seed {key}: {e}")))?;
    }

    Ok(Json(CreateBotResponse {
        name: name.clone(),
        status: "created".to_string(),
    }))
}
