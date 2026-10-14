use crate::auth::{resolve_user_id, resolve_user_name, SheetUser};
use crate::collaboration::broadcast_sheet_change;
use crate::formulas::{evaluate_formula, evaluate_formula_in};
use crate::state::SheetState;
use crate::storage::import::create_new_spreadsheet;
use crate::types::{CellData, CellUpdateRequest, FormatRequest, FormulaRequest, FormulaResult, SaveResponse, Worksheet};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use botsheet_core::dependency_graph::DepGraph;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

/// Upper bound on formulas recalculated for a single cell edit.
/// Keeps one request from monopolizing a worker thread on a pathological sheet.
pub const MAX_RECALC_CELLS: usize = 1000;

/// Bounds enforced across the grid (#786): the desktop grid is
/// 1,048,576 rows × 16,384 columns.
pub const MAX_ROWS: u32 = 1_048_576;
pub const MAX_COLS: u32 = 16_384;

fn in_bounds(row: u32, col: u32) -> bool {
    row < MAX_ROWS && col < MAX_COLS
}

pub async fn handle_update_cell(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Json(req): Json<CellUpdateRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());

    if !in_bounds(req.row, req.col) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Cell out of grid bounds" })),
        ));
    }

    let session = match state.sessions.get_or_load(&state, &user_id, &req.sheet_id).await {
        Ok(s) => s,
        Err(_) => {
            // First edit on a brand-new sheet id: materialize a fresh document
            // in Drive so the session store can load it.
            log::info!(
                "Sheet {} not found on drive, creating fresh sheet for cell update",
                req.sheet_id
            );
            let mut fresh = create_new_spreadsheet(&user_id);
            fresh.id = req.sheet_id.clone();
            if let Err(e) = crate::state::save_sheet_to_drive(&state, &user_id, &fresh).await {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::cell_ops", None)})),
                ));
            }
            match state.sessions.get_or_load(&state, &user_id, &req.sheet_id).await {
                Ok(s) => s,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::cell_ops", None)})),
                    ))
                }
            }
        }
    };

    let mut sheet = session.sheet.write().await;
    if let Err(e) = botsheet_core::state::ensure_write_allowed(&user_id, &sheet) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": e }))));
    }

    if req.worksheet_index >= sheet.worksheets.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid worksheet index" })),
        ));
    }

    let key = format!("{},{}", req.row, req.col);

    let (value, formula, typed) = if req.value.starts_with('=') {
        let result = botsheet_core::engine::evaluate_typed_in(
            &req.value,
            &sheet.worksheets,
            req.worksheet_index,
        );
        (Some(result.display()), Some(req.value.clone()), Some(result))
    } else {
        let typed = botsheet_core::engine::CellValue::parse(&req.value);
        (Some(req.value.clone()), None, Some(typed))
    };

    let worksheet = &mut sheet.worksheets[req.worksheet_index];
    let cell = worksheet.data.entry(key).or_insert_with(|| CellData {
        value: None,
        typed: None,
        formula: None,
        style: None,
        format: None,
        note: None,
        locked: None,
        has_comment: None,
        array_formula_id: None,
    });

    cell.value = value;
    cell.formula = formula;
    cell.typed = typed;

    {
        // Update the cached dependency graph for the edited cell, then
        // recalculate its dependents from the cached topology (#784).
        let mut graphs = match session.dep_graphs.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if graphs.len() != sheet.worksheets.len() {
            *graphs = sheet.worksheets.iter().map(DepGraph::build).collect();
        }
        let index = req.worksheet_index;
        graphs[index].on_edit(&sheet.worksheets[index], &[(req.row, req.col)]);
        graphs[index].recalc_cascade_typed(
            &mut sheet.worksheets[index],
            (req.row, req.col),
            MAX_RECALC_CELLS,
        );
    }

    sheet.updated_at = Utc::now();

    state.sessions.record(
        &session,
        &state,
        &user_id,
        "cell_update",
        serde_json::json!({
            "sheet_id": req.sheet_id,
            "worksheet_index": req.worksheet_index,
            "row": req.row,
            "col": req.col,
            "value": req.value,
        }),
    );

    broadcast_sheet_change(
        &req.sheet_id,
        &user_id,
        &resolve_user_name(user.as_deref()),
        req.row,
        req.col,
        &req.value,
        req.worksheet_index,
    )
    .await;

    Ok(Json(SaveResponse {
        id: req.sheet_id,
        success: true,
        message: Some("Cell updated".to_string()),
    }))
}

pub async fn handle_format_cells(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Json(req): Json<FormatRequest>,
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
    if let Err(e) = botsheet_core::state::ensure_write_allowed(&user_id, &sheet) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": e }))));
    }

    if req.worksheet_index >= sheet.worksheets.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid worksheet index" })),
        ));
    }

    let worksheet = &mut sheet.worksheets[req.worksheet_index];

    for row in req.start_row..=req.end_row {
        for col in req.start_col..=req.end_col {
            let key = format!("{},{}", row, col);
            let cell = worksheet.data.entry(key).or_insert_with(|| CellData {
                value: None,
                typed: None,
                formula: None,
                style: None,
                format: None,
                note: None,
                locked: None,
                has_comment: None,
                array_formula_id: None,
            });
            cell.style = Some(req.style.clone());
        }
    }

    sheet.updated_at = Utc::now();

    state.sessions.record(
        &session,
        &state,
        &user_id,
        "format",
        serde_json::json!({
            "sheet_id": req.sheet_id,
            "worksheet_index": req.worksheet_index,
            "start_row": req.start_row,
            "start_col": req.start_col,
            "end_row": req.end_row,
            "end_col": req.end_col,
        }),
    );

    Ok(Json(SaveResponse {
        id: req.sheet_id,
        success: true,
        message: Some("Format applied".to_string()),
    }))
}

pub async fn handle_evaluate_formula(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Json(req): Json<FormulaRequest>,
) -> Result<Json<FormulaResult>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());

    let sheet = match state.sessions.get_or_load(&state, &user_id, &req.sheet_id).await {
        Ok(s) => {
            let sheet = s.sheet.read().await;
            sheet.clone()
        }
        Err(_) => {
            return Ok(Json(evaluate_formula(
                &req.formula,
                &Worksheet {
                    name: "temp".to_string(),
                    data: HashMap::new(),
                    column_widths: None,
                    row_heights: None,
                    frozen_rows: None,
                    frozen_cols: None,
                    merged_cells: None,
                    filters: None,
                    hidden_rows: None,
                    validations: None,
                    conditional_formats: None,
                    charts: None,
                    comments: None,
                    protection: None,
                    array_formulas: None,
                    tables: None,
                    hidden_columns: None,
                    sheet_state: None,
                    hyperlinks: None,
                    print_setup: None,
                    autofilter: None,
                    row_page_breaks: None,
                    column_page_breaks: None,
                    images: None,
                    print_areas: None,
                    rich_text: None,
                },
            )))
        }
    };

    if req.worksheet_index >= sheet.worksheets.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid worksheet index" })),
        ));
    }

    let result = evaluate_formula_in(&req.formula, &sheet.worksheets, req.worksheet_index);
    Ok(Json(result))
}
