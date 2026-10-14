use crate::state::DocState;
use crate::storage::{
    create_new_document, delete_document_from_drive, get_current_user_id,
    list_documents_from_drive, load_document_from_drive, save_document_to_drive,
};
use crate::types::{DocsSaveRequest, DocsSaveResponse, Document, DocumentMetadata, SearchQuery};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

pub async fn handle_docs_save(
    State(state): State<Arc<DocState>>,
    Json(req): Json<DocsSaveRequest>,
) -> Result<Json<DocsSaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let doc_id = req.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    // #1138: archive the previous version before overwriting it.
    super::history::capture_snapshot(&state, &doc_id, &req.title, &req.content).await;

    if let Err(e) =
        save_document_to_drive(&state, &user_id, &doc_id, &req.title, &req.content).await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::crud", None)})),
        ));
    }

    Ok(Json(DocsSaveResponse {
        id: doc_id,
        success: true,
    }))
}

pub async fn handle_docs_get_by_id(
    State(state): State<Arc<DocState>>,
    Path(doc_id): Path<String>,
) -> Result<Json<Document>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    match load_document_from_drive(&state, &user_id, &doc_id).await {
        Ok(Some(doc)) => Ok(Json(doc)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Document not found" })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::crud", None)})),
        )),
    }
}

pub async fn handle_new_document(
    State(_state): State<Arc<DocState>>,
) -> Result<Json<Document>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(create_new_document()))
}

pub async fn handle_list_documents(
    State(state): State<Arc<DocState>>,
) -> Result<Json<Vec<DocumentMetadata>>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    match list_documents_from_drive(&state, &user_id).await {
        Ok(docs) => Ok(Json(docs)),
        Err(e) => {
            log::error!("Failed to list documents: {}", e);
            Ok(Json(Vec::new()))
        }
    }
}

pub async fn handle_search_documents(
    State(state): State<Arc<DocState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<DocumentMetadata>>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let docs = match list_documents_from_drive(&state, &user_id).await {
        Ok(d) => d,
        Err(_) => Vec::new(),
    };

    let filtered = if let Some(q) = query.q {
        let q_lower = q.to_lowercase();
        docs.into_iter()
            .filter(|d| d.title.to_lowercase().contains(&q_lower))
            .collect()
    } else {
        docs
    };

    Ok(Json(filtered))
}

pub async fn handle_get_document(
    State(state): State<Arc<DocState>>,
    Query(query): Query<crate::types::LoadQuery>,
) -> Result<Json<Document>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let doc_id = query.id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Document ID required" })),
        )
    })?;

    match load_document_from_drive(&state, &user_id, &doc_id).await {
        Ok(Some(doc)) => Ok(Json(doc)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Document not found" })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::crud", None)})),
        )),
    }
}

pub async fn handle_save_document(
    State(state): State<Arc<DocState>>,
    Json(req): Json<DocsSaveRequest>,
) -> Result<Json<DocsSaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let doc_id = req.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    if let Err(e) =
        save_document_to_drive(&state, &user_id, &doc_id, &req.title, &req.content).await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::crud", None)})),
        ));
    }

    Ok(Json(DocsSaveResponse {
        id: doc_id,
        success: true,
    }))
}

pub async fn handle_autosave(
    State(state): State<Arc<DocState>>,
    Json(req): Json<DocsSaveRequest>,
) -> Result<Json<DocsSaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    handle_save_document(State(state), Json(req)).await
}

pub async fn handle_delete_document(
    State(state): State<Arc<DocState>>,
    Json(req): Json<crate::types::LoadQuery>,
) -> Result<Json<DocsSaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let doc_id = req.id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Document ID required" })),
        )
    })?;

    if let Err(e) = delete_document_from_drive(&state, &user_id, &doc_id).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::crud", None)})),
        ));
    }

    Ok(Json(DocsSaveResponse {
        id: doc_id,
        success: true,
    }))
}

/// Open an arbitrary file from the Drive (any bucket) inside the docs editor.
///
/// Reads the object bytes from MinIO/S3 and converts binary office formats
/// (.docx/.doc/.odt/.rtf) to HTML so the content is editable. Plain text
/// files (.txt/.md/.html) are used as-is.
pub async fn handle_open_from_drive(
    State(state): State<Arc<DocState>>,
    Json(req): Json<crate::types::OpenFromDriveRequest>,
) -> Result<Json<Document>, (StatusCode, Json<serde_json::Value>)> {
    if req.bucket.is_empty() || req.path.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "bucket and path are required" })),
        ));
    }

    let bytes = match state.drive.get_object(&req.bucket, &req.path).await {
        Ok(b) => b,
        Err(e) => {
            // The drive error text names the bucket and object path; the client
            // only needs the lookup result. The detail goes to the log.
            log::warn!("docs editor: drive read of {}/{} failed: {}", req.bucket, req.path, e);
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "File not found in drive" })),
            ))
        }
    };

    let file_name = req
        .path
        .rsplit('/')
        .next()
        .unwrap_or(&req.path)
        .to_string();
    let ext = file_name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();

    let content = match ext.as_str() {
        "docx" | "doc" | "odt" | "rtf" => match crate::storage::convert_docx_to_html(&bytes) {
            Ok(html) => html,
            Err(e) => {
                log::error!("Failed to convert {} to HTML: {}", req.path, e);
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::crud", None) })),
                ));
            }
        },
        "txt" | "md" | "html" | "htm" | "json" | "xml" | "csv" => {
            String::from_utf8_lossy(&bytes).into_owned()
        }
        _ => {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "error": format!("Unsupported file type for docs editor: .{ext}") })),
            ))
        }
    };

    let now = chrono::Utc::now();
    let title = file_name
        .rsplit('.')
        .next_back()
        .map(|stem| file_name.trim_end_matches(&format!(".{stem}")).to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(file_name.clone());

    Ok(Json(Document {
        id: format!("drive-{}", req.path.replace(['/', '.', ':'], "-")),
        title,
        content,
        owner_id: get_current_user_id(),
        storage_path: format!("{}/{}", req.bucket, req.path),
        created_at: now,
        updated_at: now,
        collaborators: Vec::new(),
        version: 1,
        track_changes: None,
        comments: None,
        footnotes: None,
        endnotes: None,
        styles: None,
        toc: None,
        track_changes_enabled: false,
    }))
}
