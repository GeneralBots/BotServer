//! Split from `crud.rs` — 251 lines.

use super::*;

pub async fn handle_save_sheet(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Json(req): Json<SaveRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());
    let action = req.action.as_deref().unwrap_or("save");

    if action == "rename" {
        let sheet_id = req.id.as_deref().ok_or_else(|| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "error": "rename requires a sheet id" })),
            )
        })?;
        let name = req.name.as_deref().ok_or_else(|| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "error": "rename requires a name" })),
            )
        })?;
        let session = state
            .sessions
            .get_or_load(&state, &user_id, sheet_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": e })),
                )
            })?;
        {
            let mut sheet = session.sheet.write().await;
            sheet.name = name.to_string();
            sheet.updated_at = Utc::now();
        }
        state.sessions.record(
            &session,
            &state,
            &user_id,
            "rename",
            serde_json::json!({ "sheet_id": sheet_id, "name": name }),
        );
        return Ok(Json(SaveResponse {
            id: sheet_id.to_string(),
            success: true,
            message: Some("Sheet renamed".to_string()),
        }));
    }

    if req.worksheets.is_empty() {
        // No full snapshot: flush the live session (authoritative), or create
        // a blank document — the client never sends a viewport-only dump.
        if let Some(sheet_id) = req.id.as_deref() {
            let session = state
                .sessions
                .get_or_load(&state, &user_id, sheet_id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({ "error": e })),
                    )
                })?;
            {
                let mut sheet = session.sheet.write().await;
                if let Some(name) = req.name.as_deref() {
                    sheet.name = name.to_string();
                }
                sheet.updated_at = Utc::now();
            }
            if let Err(e) = session.persist_now(&state).await {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(
                        serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::crud", None)}),
                    ),
                ));
            }
            return Ok(Json(SaveResponse {
                id: sheet_id.to_string(),
                success: true,
                message: Some("Sheet saved".to_string()),
            }));
        }

        let mut sheet = create_new_spreadsheet(&user_id);
        sheet.name = req
            .name
            .unwrap_or_else(|| "Untitled Spreadsheet".to_string());
        sheet.updated_at = Utc::now();
        let sheet_id = sheet.id.clone();
        if let Err(e) = save_sheet_to_drive(&state, &user_id, &sheet).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::crud", None)}),
                ),
            ));
        }
        return Ok(Json(SaveResponse {
            id: sheet_id,
            success: true,
            message: Some("Sheet saved".to_string()),
        }));
    }

    let sheet_id = req.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let sheet = Spreadsheet {
        id: sheet_id.clone(),
        name: req
            .name
            .unwrap_or_else(|| "Untitled Spreadsheet".to_string()),
        owner_id: user_id.clone(),
        worksheets: req.worksheets,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        named_ranges: None,
        external_links: None,
        source_bucket: None,
        source_path: None,
        source_bytes: None,
        acl: HashMap::new(),
    };

    if let Err(e) = save_sheet_to_drive(&state, &user_id, &sheet).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::crud", None)}),
            ),
        ));
    }

    Ok(Json(SaveResponse {
        id: sheet_id,
        success: true,
        message: Some("Sheet saved successfully".to_string()),
    }))
}

pub async fn handle_delete_sheet(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Json(req): Json<LoadQuery>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());

    if let Err(e) = delete_sheet_from_drive(&state, &user_id, &req.id).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::crud", None)}),
            ),
        ));
    }
    if let Some(id) = req.id.as_ref() {
        state.sessions.close(&state, id).await;
    }

    Ok(Json(SaveResponse {
        id: req.id.unwrap_or_default(),
        success: true,
        message: Some("Sheet deleted".to_string()),
    }))
}

pub async fn handle_get_sheet_by_id(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    headers: HeaderMap,
    Path(sheet_id): Path<String>,
) -> Result<Json<Spreadsheet>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());
    let session = state
        .sessions
        .get_or_load(&state, &user_id, &sheet_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;
    let mut sheet = session.sheet.read().await.clone();
    botsheet_core::engine::formats::apply_formats_to_sheet_locale(
        &mut sheet,
        Lang::from_headers(&headers).number_locale(),
    );
    Ok(Json(sheet))
}

/// Returns the ops after `since` (0 = all) plus the version, so a reconnecting
/// client can converge with the session state (#789, #791).
pub async fn handle_sheet_ops(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Path(sheet_id): Path<String>,
    Query(q): Query<OpsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());
    let session = state
        .sessions
        .get_or_load(&state, &user_id, &sheet_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;

    let version = session.version.load(std::sync::atomic::Ordering::SeqCst);
    Ok(Json(serde_json::json!({
        "version": version,
        "ops": session.ops_since(q.since),
    })))
}

#[derive(serde::Deserialize, Default)]
pub struct OpsQuery {
    #[serde(default)]
    pub since: u64,
}

pub async fn handle_share_sheet(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Json(req): Json<ShareRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());

    let session = state
        .sessions
        .get_or_load(&state, &user_id, &req.sheet_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;

    let mut sheet = session.sheet.write().await;
    if sheet.owner_id != user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Only the sheet owner can share it" })),
        ));
    }

    // Share by explicit user id when provided, otherwise key the grant by the
    // email so a future email-to-id resolution can migrate it (#789).
    let grant_key = req.user_id.unwrap_or_else(|| req.email.clone());
    sheet.acl.insert(grant_key, req.permission.clone());

    sheet.updated_at = Utc::now();
    state.sessions.record(&session, &state, &user_id, "share", serde_json::json!({ "sheet_id": req.sheet_id, "email": req.email, "permission": req.permission }));

    Ok(Json(SaveResponse {
        id: req.sheet_id,
        success: true,
        message: Some(format!("Shared with {} as {}", req.email, req.permission)),
    }))
}
