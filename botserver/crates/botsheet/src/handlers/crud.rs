use crate::auth::{resolve_user_id, SheetUser};
use crate::state::{delete_sheet_from_drive, list_sheets_from_drive, load_sheet_from_drive,
    persist_sheet_to_drive, save_sheet_to_drive, SheetState};
use crate::storage::import::{create_new_spreadsheet, parse_csv_to_worksheets, parse_xlsx_to_worksheets};
use crate::types::{LoadFromDriveRequest, LoadQuery, SaveRequest, SaveResponse, SearchQuery,
    ShareRequest, Spreadsheet, SpreadsheetMetadata};
use crate::ui_fragments::Lang;
use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use sha2::Digest;

use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub async fn handle_new_sheet(
    State(_state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
) -> Result<Json<Spreadsheet>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());
    Ok(Json(create_new_spreadsheet(&user_id)))
}

pub async fn handle_list_sheets(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
) -> Result<Json<Vec<SpreadsheetMetadata>>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());

    match list_sheets_from_drive(&state, &user_id).await {
        Ok(sheets) => Ok(Json(sheets)),
        Err(e) => {
            log::error!("Failed to list sheets: {e}");
            Ok(Json(Vec::new()))
        }
    }
}

pub async fn handle_search_sheets(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SpreadsheetMetadata>>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());

    let sheets = list_sheets_from_drive(&state, &user_id).await.unwrap_or_default();

    let filtered = if let Some(q) = query.q {
        let q_lower = q.to_lowercase();
        sheets
            .into_iter()
            .filter(|s| s.name.to_lowercase().contains(&q_lower))
            .collect()
    } else {
        sheets
    };

    Ok(Json(filtered))
}

pub async fn handle_load_sheet(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    headers: HeaderMap,
    Query(query): Query<LoadQuery>,
) -> Result<Json<Spreadsheet>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());

    let sheet_id = query.id.as_deref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Sheet ID is required" })),
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

    let mut sheet = session.sheet.read().await.clone();
    botsheet_core::engine::formats::apply_formats_to_sheet_locale(&mut sheet, Lang::from_headers(&headers).number_locale());
    Ok(Json(sheet))
}

pub async fn handle_load_from_drive(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    headers: HeaderMap,
    Json(req): Json<LoadFromDriveRequest>,
) -> Result<Json<Spreadsheet>, (StatusCode, Json<serde_json::Value>)> {
    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive not available" })),
        )
    })?;

    let bytes = drive.get_object(&req.bucket, &req.path).await.map_err(|e| {
        // The drive error names the bucket and object path — log it and answer
        // with the lookup result only.
        log::warn!("sheet load: drive read of {}/{} failed: {}", req.bucket, req.path, e);
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "File not found" })),
        )
    })?;

    let mut ext = req.path.rsplit('.').next().unwrap_or("").to_lowercase();
    // Detect actual format from magic bytes for misnamed files
    if bytes.len() >= 4 && &bytes[..4] == b"PK\x03\x04" && ext != "ods" && ext != "xlsx" && ext != "xls" {
        ext = "xlsx".to_string();
    }
    let file_name = req.path.rsplit('/').next().unwrap_or("Spreadsheet");
    let sheet_name = file_name
        .rsplit('.')
        .next_back()
        .unwrap_or("Spreadsheet")
        .to_string();

    let worksheets = match ext.as_str() {
        "csv" | "tsv" => {
            let delimiter = if ext == "tsv" { b'\t' } else { b',' };
            parse_csv_to_worksheets(&bytes, delimiter, &sheet_name).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": e })),
                )
            })?
        }
        "xlsx" | "xls" | "ods" | "xlsb" | "xlsm" => {
            parse_xlsx_to_worksheets(&bytes, &ext).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": e })),
                )
            })?
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unsupported format: .{ext}") })),
            ));
        }
    };

    let user_id = resolve_user_id(user.as_deref());
    // Deterministic working-copy id: SHA-256(bucket|path) truncated to a v4
    // UUID, so reopening the same source reuses its prior edits.
    let digest = sha2::Sha256::digest(format!("{}|{}", req.bucket, req.path).as_bytes());
    let mut id_bytes = [0u8; 16];
    id_bytes.copy_from_slice(&digest[..16]);
    let sheet_id = Uuid::from_bytes(id_bytes).to_string();

    // Reuse an existing working copy (with any prior edits) when present.
    let locale = Lang::from_headers(&headers).number_locale();
    if let Ok(mut existing) = load_sheet_from_drive(&state, &user_id, &Some(sheet_id.clone())).await
    {
        botsheet_core::engine::formats::apply_formats_to_sheet_locale(&mut existing, locale);
        return Ok(Json(existing));
    }

    let sheet = Spreadsheet {
        id: sheet_id.clone(),
        name: sheet_name,
        owner_id: user_id.clone(),
        worksheets,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        named_ranges: None,
        external_links: None,
        source_bucket: Some(req.bucket.clone()),
        source_path: Some(req.path.clone()),
        // Retain the original package for the save-back merge (#788).
        source_bytes: if ext == "xlsx" || ext == "xlsm" {
            Some(bytes.clone())
        } else {
            None
        },
        acl: HashMap::new(),
    };

    // Persist (no export hook — opening must not rewrite the source).
    let _ = persist_sheet_to_drive(&state, &user_id, &sheet).await;

    // Return a display-formatted clone; the persisted model keeps raw values.
    let mut response = sheet.clone();
    botsheet_core::engine::formats::apply_formats_to_sheet_locale(&mut response, locale);
    Ok(Json(response))
}

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
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::crud", None)})),
                ));
            }
            return Ok(Json(SaveResponse {
                id: sheet_id.to_string(),
                success: true,
                message: Some("Sheet saved".to_string()),
            }));
        }

        let mut sheet = create_new_spreadsheet(&user_id);
        sheet.name = req.name.unwrap_or_else(|| "Untitled Spreadsheet".to_string());
        sheet.updated_at = Utc::now();
        let sheet_id = sheet.id.clone();
        if let Err(e) = save_sheet_to_drive(&state, &user_id, &sheet).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::crud", None)})),
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
        name: req.name.unwrap_or_else(|| "Untitled Spreadsheet".to_string()),
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
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::crud", None)})),
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
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::crud", None)})),
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
    botsheet_core::engine::formats::apply_formats_to_sheet_locale(&mut sheet, Lang::from_headers(&headers).number_locale());
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
