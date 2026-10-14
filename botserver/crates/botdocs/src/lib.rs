pub mod collaboration;
pub mod handlers_api;
pub mod ooxml;
pub mod review;
pub mod state;
pub mod storage;
pub mod storage_core;
pub mod storage_docx;
pub mod storage_drive;
pub mod types;
pub mod types_core;
pub mod types_requests;
pub mod ui_fragments;
pub mod utils;
pub mod utils_convert;
pub mod utils_format;
pub mod utils_ooxml;
pub mod utils_pdf;

mod re_cache;

use axum::{routing::get, Router};
use state::DocState;
use std::sync::Arc;

pub use collaboration::{
    handle_docs_websocket, handle_get_collaborators, handle_get_mentions, handle_get_presence,
    handle_get_selections, handle_get_typing,
};
pub use handlers_api::*;
pub use review::{MergedFinding, ReviewOutcome, ReviewerRole, Severity};
pub use types::{
    AiRequest, AiResponse, CollabMessage, CommentReply, ComparisonSummary, Document,
    DocumentComment, DocumentComparison, DocumentDiff, DocumentMetadata, DocumentStyle, Endnote,
    Footnote, OutlineItem, SaveRequest, SaveResponse, SearchQuery, TableOfContents, TocEntry,
    TrackChange,
};
pub use types_requests::*;

pub fn configure_docs_routes() -> Router<Arc<DocState>> {
    Router::new()
        .merge(ui_fragments::configure())
        .route("/api/docs/list", get(handle_list_documents))
        .route("/api/docs/search", get(handle_search_documents))
        .route("/api/docs/load", get(handle_get_document))
        .route("/api/docs/save", axum::routing::post(handle_docs_save))
        .route("/api/docs/autosave", axum::routing::post(handle_autosave))
        .route("/api/docs/history", axum::routing::post(handlers_api::history::handle_history))
        .route("/api/docs/delete", axum::routing::post(handle_delete_document))
        .route("/api/docs/new", get(handle_new_document))
        .route("/api/docs/ai", axum::routing::post(handle_docs_ai))
        .route("/api/docs/:id", get(handle_docs_get_by_id))
        .route("/api/docs/template/blank", get(handle_template_blank))
        .route("/api/docs/template/meeting", get(handle_template_meeting))
        .route("/api/docs/template/report", get(handle_template_report))
        .route("/api/docs/template/letter", get(handle_template_letter))
        .route("/api/docs/ai/summarize", axum::routing::post(handle_ai_summarize))
        .route("/api/docs/ai/expand", axum::routing::post(handle_ai_expand))
        .route("/api/docs/ai/improve", axum::routing::post(handle_ai_improve))
        .route("/api/docs/ai/review-pair", axum::routing::post(handle_ai_review_pair))
        .route("/api/docs/ai/simplify", axum::routing::post(handle_ai_simplify))
        .route("/api/docs/ai/translate", axum::routing::post(handle_ai_translate))
        .route("/api/docs/ai/custom", axum::routing::post(handle_ai_custom))
        .route("/api/docs/export/pdf", get(handle_export_pdf))
        .route("/api/docs/export/docx", get(handle_export_docx))
        .route("/api/docs/export/odt", get(handle_export_odt))
        .route("/api/docs/export/md", get(handle_export_md))
        .route("/api/docs/export/html", get(handle_export_html))
        .route("/api/docs/export/txt", get(handle_export_txt))
        .route("/api/docs/import", axum::routing::post(handle_import_document))
        .route("/api/docs/open-from-drive", axum::routing::post(handle_open_from_drive))
        .route("/api/docs/comment", axum::routing::post(handle_add_comment))
        .route("/api/docs/comment/reply", axum::routing::post(handle_reply_comment))
        .route("/api/docs/comment/resolve", axum::routing::post(handle_resolve_comment))
        .route("/api/docs/comment/delete", axum::routing::post(handle_delete_comment))
        .route("/api/docs/comments", get(handle_list_comments))
        .route("/api/docs/track-changes/enable", axum::routing::post(handle_enable_track_changes))
        .route("/api/docs/track-changes/accept-reject", axum::routing::post(handle_accept_reject_change))
        .route("/api/docs/track-changes/accept-reject-all", axum::routing::post(handle_accept_reject_all))
        .route("/api/docs/track-changes", get(handle_list_track_changes))
        .route("/api/docs/toc/generate", axum::routing::post(handle_generate_toc))
        .route("/api/docs/toc/update", axum::routing::post(handle_update_toc))
        .route("/api/docs/footnote", axum::routing::post(handle_add_footnote))
        .route("/api/docs/footnote/update", axum::routing::post(handle_update_footnote))
        .route("/api/docs/footnote/delete", axum::routing::post(handle_delete_footnote))
        .route("/api/docs/footnotes", get(handle_list_footnotes))
        .route("/api/docs/endnote", axum::routing::post(handle_add_endnote))
        .route("/api/docs/endnote/update", axum::routing::post(handle_update_endnote))
        .route("/api/docs/endnote/delete", axum::routing::post(handle_delete_endnote))
        .route("/api/docs/endnotes", get(handle_list_endnotes))
        .route("/api/docs/style", axum::routing::post(handle_create_style))
        .route("/api/docs/style/update", axum::routing::post(handle_update_style))
        .route("/api/docs/style/delete", axum::routing::post(handle_delete_style))
        .route("/api/docs/style/apply", axum::routing::post(handle_apply_style))
        .route("/api/docs/styles", get(handle_list_styles))
        .route("/api/docs/outline", axum::routing::post(handle_get_outline))
        .route("/api/docs/compare", axum::routing::post(handle_compare_documents))
        .route("/api/docs/:doc_id/collaborators", get(handle_get_collaborators))
        .route("/api/docs/:doc_id/presence", get(handle_get_presence))
        .route("/api/docs/:doc_id/typing", get(handle_get_typing))
        .route("/api/docs/:doc_id/selections", get(handle_get_selections))
        .route("/api/docs/mentions/:user_id", get(handle_get_mentions))
        .route("/ws/docs/:doc_id", get(handle_docs_websocket))
}
