use crate::collaboration::{
handle_get_collaborators, handle_get_mentions, handle_get_presence, handle_get_selections,
handle_get_typing, handle_sheet_websocket,
};
use crate::handlers::advanced::{
    handle_add_external_link, handle_list_external_links, handle_lock_cells,
    handle_protect_sheet, handle_refresh_external_link, handle_remove_external_link,
    handle_unprotect_sheet,
};
use crate::handlers::advanced_ranges::{
    handle_array_formula, handle_create_named_range, handle_delete_array_formula,
    handle_delete_named_range, handle_list_named_ranges, handle_update_named_range,
};
use crate::handlers::ai::{handle_batch_evaluate_ai, handle_evaluate_ai_prompt, handle_sheet_ai};
use crate::handlers::cell_ops::{
    handle_evaluate_formula, handle_format_cells, handle_update_cell,
};
use crate::handlers::cell_structure::{
    handle_freeze_panes, handle_get_range, handle_merge_cells, handle_unmerge_cells,
    handle_worksheet_meta,
};
use crate::handlers::comments::{
    handle_add_comment, handle_delete_comment, handle_list_comments, handle_reply_comment,
    handle_resolve_comment,
};
use crate::handlers::crud::{
    handle_delete_sheet, handle_get_sheet_by_id, handle_list_sheets, handle_load_from_drive,
    handle_load_sheet, handle_new_sheet, handle_save_sheet, handle_search_sheets,
    handle_share_sheet, handle_sheet_ops,
};
use crate::handlers::data_ops::{
    handle_clear_filter, handle_conditional_format, handle_create_chart, handle_delete_chart,
    handle_filter_data, handle_sort_range,
};
use crate::handlers::export_import::{handle_export_sheet, handle_import_sheet};
use crate::handlers::grid_ops::{
    handle_paste, handle_resize, handle_table_create, handle_table_delete, handle_table_list,
};
use crate::handlers::named_ranges_csv::{
    handle_export_named_ranges_csv, handle_import_named_ranges_csv,
};
use crate::handlers::pivot::handle_pivot;
use crate::handlers::validation::{handle_add_note, handle_data_validation, handle_validate_cell};
use crate::handlers::worksheets::{
add_worksheet, delete_worksheet, rename_worksheet, switch_worksheet,
};
use botsheet_core::state::SheetState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn configure_sheet_routes() -> Router<Arc<SheetState>> {
    Router::new()
        .merge(crate::ui_fragments::configure())
        .route("/api/sheet/list", get(handle_list_sheets))
        .route("/api/sheet/search", get(handle_search_sheets))
        .route("/api/sheet/load", get(handle_load_sheet))
        .route("/api/sheet/load-from-drive", post(handle_load_from_drive))
        .route("/api/sheet/save", post(handle_save_sheet))
        .route("/api/sheet/delete", post(handle_delete_sheet))
        .route("/api/sheet/cell", post(handle_update_cell))
        .route("/api/sheet/format", post(handle_format_cells))
        .route("/api/sheet/formula", post(handle_evaluate_formula))
        .route("/api/sheet/range", post(handle_get_range))
        .route("/api/sheet/worksheet-meta", post(handle_worksheet_meta))
        .route("/api/sheet/worksheets/add", post(add_worksheet))
        .route("/api/sheet/worksheets/switch", post(switch_worksheet))
        .route("/api/sheet/worksheets/delete", post(delete_worksheet))
        .route("/api/sheet/worksheets/rename", post(rename_worksheet))
        .route("/api/sheet/pivot", post(handle_pivot))
        .route("/api/sheet/export", post(handle_export_sheet))
        .route("/api/sheet/share", post(handle_share_sheet))
        .route("/api/sheet/new", get(handle_new_sheet))
        .route("/api/sheet/merge", post(handle_merge_cells))
        .route("/api/sheet/unmerge", post(handle_unmerge_cells))
        .route("/api/sheet/freeze", post(handle_freeze_panes))
        .route("/api/sheet/sort", post(handle_sort_range))
        .route("/api/sheet/filter", post(handle_filter_data))
        .route("/api/sheet/filter/clear", post(handle_clear_filter))
        .route("/api/sheet/chart", post(handle_create_chart))
        .route("/api/sheet/chart/delete", post(handle_delete_chart))
        .route("/api/sheet/conditional-format", post(handle_conditional_format))
        .route("/api/sheet/data-validation", post(handle_data_validation))
        .route("/api/sheet/validate-cell", post(handle_validate_cell))
        .route("/api/sheet/note", post(handle_add_note))
        .route("/api/sheet/import", post(handle_import_sheet))
        .route("/api/sheet/ai", post(handle_sheet_ai))
        .route("/api/sheet/ai/evaluate-prompt", post(handle_evaluate_ai_prompt))
        .route("/api/sheet/ai/batch-evaluate", post(handle_batch_evaluate_ai))
        .route("/api/sheet/:id", get(handle_get_sheet_by_id))
        .route("/api/sheet/:id/ops", get(handle_sheet_ops))
        .route("/api/sheet/:id/collaborators", get(handle_get_collaborators))
        .route("/api/sheet/comment", post(handle_add_comment))
        .route("/api/sheet/comment/reply", post(handle_reply_comment))
        .route("/api/sheet/comment/resolve", post(handle_resolve_comment))
        .route("/api/sheet/comment/delete", post(handle_delete_comment))
        .route("/api/sheet/comments", post(handle_list_comments))
        .route("/api/sheet/protect", post(handle_protect_sheet))
        .route("/api/sheet/unprotect", post(handle_unprotect_sheet))
        .route("/api/sheet/lock-cells", post(handle_lock_cells))
        .route("/api/sheet/external-link", post(handle_add_external_link))
        .route("/api/sheet/external-link/refresh", post(handle_refresh_external_link))
        .route("/api/sheet/external-link/remove", post(handle_remove_external_link))
        .route("/api/sheet/external-links", get(handle_list_external_links))
        .route("/api/sheet/array-formula", post(handle_array_formula))
        .route("/api/sheet/array-formula/delete", post(handle_delete_array_formula))
        .route("/api/sheet/resize", post(handle_resize))
        .route("/api/sheet/paste", post(handle_paste))
        .route("/api/sheet/tables", get(handle_table_list))
        .route("/api/sheet/table", post(handle_table_create))
        .route("/api/sheet/table/delete", post(handle_table_delete))
        .route("/api/sheet/named-range", post(handle_create_named_range))
        .route("/api/sheet/named-range/update", post(handle_update_named_range))
        .route("/api/sheet/named-range/delete", post(handle_delete_named_range))
        .route("/api/sheet/named-ranges", get(handle_list_named_ranges))
        .route("/api/sheet/named-ranges/export", get(handle_export_named_ranges_csv))
        .route("/api/sheet/named-ranges/import", post(handle_import_named_ranges_csv))
        .route("/api/sheet/:sheet_id/presence", get(handle_get_presence))
        .route("/api/sheet/:sheet_id/typing", get(handle_get_typing))
        .route("/api/sheet/:sheet_id/selections", get(handle_get_selections))
        .route("/api/sheet/mentions/:user_id", get(handle_get_mentions))
        .route("/ws/sheet/:sheet_id", get(handle_sheet_websocket))
}
