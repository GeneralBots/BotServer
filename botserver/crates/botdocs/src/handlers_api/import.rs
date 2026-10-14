use crate::state::DocState;
use crate::storage::{create_new_document, get_current_user_id, load_document_from_drive, save_document};
use crate::types::{
    CompareDocumentsRequest, CompareDocumentsResponse, ComparisonSummary, Document,
    DocumentComparison, DocumentDiff,
};
use crate::storage::convert_docx_to_html;
use crate::utils::{detect_document_format, markdown_to_html, rtf_to_html, strip_html};
use crate::utils_ooxml::odt_zip_to_html;
use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use std::sync::Arc;

pub async fn handle_import_document(
    State(state): State<Arc<DocState>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<Document>, (StatusCode, Json<serde_json::Value>)> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = "import.docx".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field.file_name().unwrap_or("import.docx").to_string();
            if let Ok(bytes) = field.bytes().await {
                file_bytes = Some(bytes.to_vec());
            }
        }
    }

    let bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No file uploaded" })),
        )
    })?;

    let format = detect_import_format(&filename, &bytes);
    let content = match format.as_str() {
        "docx" => convert_docx_to_html(&bytes).map_err(|e| {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e })))
        })?,
        "odt" => odt_zip_to_html(&bytes).map_err(|e| {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e })))
        })?,
        "rtf" => rtf_to_html(&String::from_utf8_lossy(&bytes)),
        "html" => String::from_utf8_lossy(&bytes).to_string(),
        "markdown" => markdown_to_html(&String::from_utf8_lossy(&bytes)),
        "txt" => {
            let text = String::from_utf8_lossy(&bytes);
            format!("<p>{}</p>", text.replace('\n', "</p><p>"))
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unsupported format: {format}") })),
            ))
        }
    };

    let title = filename
        .rsplit('/')
        .next()
        .unwrap_or(&filename)
        .rsplit('.')
        .next_back()
        .unwrap_or(&filename)
        .to_string();

    let user_id = get_current_user_id();
    let mut doc = create_new_document();
    doc.title = title;
    doc.content = content;
    doc.owner_id = user_id.clone();

    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::import", None)})),
        ));
    }

    Ok(Json(doc))
}

fn detect_import_format(filename: &str, bytes: &[u8]) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "docx" => return "docx".to_string(),
        "odt" => return "odt".to_string(),
        "rtf" => return "rtf".to_string(),
        "md" | "markdown" => return "markdown".to_string(),
        "html" | "htm" => return "html".to_string(),
        "txt" => return "txt".to_string(),
        _ => {}
    }
    detect_document_format(bytes).to_string()
}

pub async fn handle_compare_documents(
    State(state): State<Arc<DocState>>,
    Json(req): Json<CompareDocumentsRequest>,
) -> Result<Json<CompareDocumentsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let original = match load_document_from_drive(&state, &user_id, &req.original_doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Original document not found" })),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::import", None)})),
            ))
        }
    };

    let modified = match load_document_from_drive(&state, &user_id, &req.modified_doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Modified document not found" })),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::import", None)})),
            ))
        }
    };

    let original_text = strip_html(&original.content);
    let modified_text = strip_html(&modified.content);

    let mut differences = Vec::new();
    let mut insertions = 0u32;
    let mut deletions = 0u32;
    let mut modifications = 0u32;

    let original_words: Vec<&str> = original_text.split_whitespace().collect();
    let modified_words: Vec<&str> = modified_text.split_whitespace().collect();

    let mut i = 0;
    let mut j = 0;
    let mut position = 0;

    while i < original_words.len() || j < modified_words.len() {
        if i >= original_words.len() {
            differences.push(DocumentDiff {
                diff_type: "insertion".to_string(),
                position,
                original_text: None,
                modified_text: Some(modified_words[j].to_string()),
                length: modified_words[j].len(),
            });
            insertions += 1;
            j += 1;
        } else if j >= modified_words.len() {
            differences.push(DocumentDiff {
                diff_type: "deletion".to_string(),
                position,
                original_text: Some(original_words[i].to_string()),
                modified_text: None,
                length: original_words[i].len(),
            });
            deletions += 1;
            i += 1;
        } else if original_words[i] == modified_words[j] {
            position += original_words[i].len() + 1;
            i += 1;
            j += 1;
        } else {
            differences.push(DocumentDiff {
                diff_type: "modification".to_string(),
                position,
                original_text: Some(original_words[i].to_string()),
                modified_text: Some(modified_words[j].to_string()),
                length: original_words[i].len().max(modified_words[j].len()),
            });
            modifications += 1;
            position += modified_words[j].len() + 1;
            i += 1;
            j += 1;
        }
    }

    let comparison = DocumentComparison {
        id: uuid::Uuid::new_v4().to_string(),
        original_doc_id: req.original_doc_id,
        modified_doc_id: req.modified_doc_id,
        created_at: Utc::now(),
        differences,
        summary: ComparisonSummary {
            insertions,
            deletions,
            modifications,
            total_changes: insertions + deletions + modifications,
        },
    };

    Ok(Json(CompareDocumentsResponse { comparison }))
}
