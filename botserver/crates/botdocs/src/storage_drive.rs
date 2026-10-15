use chrono::{DateTime, Utc};

use crate::state::DocState;
use crate::storage_core::{
    cache_document_bytes, count_words, get_cached_document_bytes,
    get_user_docs_path, html_to_paragraphs, remove_from_cache,
};
use crate::storage_docx::convert_html_to_docx;
use crate::types::{Document, DocumentMetadata};

pub async fn save_document_as_docx(
    state: &DocState,
    user_identifier: &str,
    doc_id: &str,
    title: &str,
    content: &str,
) -> Result<Vec<u8>, String> {
    let docx_bytes = if let Some(original_bytes) = get_cached_document_bytes(doc_id).await {
        let paragraphs = html_to_paragraphs(content);
        crate::ooxml::update_docx_text(&original_bytes, &paragraphs)
            .unwrap_or_else(|_| convert_html_to_docx(title, content).unwrap_or_default())
    } else {
        convert_html_to_docx(title, content)?
    };

    let base_path = get_user_docs_path(user_identifier);
    let docx_path = format!("{base_path}/{doc_id}.docx");

    state
        .drive
        .put_object(
            &state.bucket_name,
            &docx_path,
            docx_bytes.clone(),
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        )
        .await
        .map_err(|e| format!("Failed to save DOCX: {e}"))?;

    cache_document_bytes(doc_id, docx_bytes.clone()).await;

    Ok(docx_bytes)
}

pub async fn save_document_to_drive(
    state: &DocState,
    user_identifier: &str,
    doc_id: &str,
    title: &str,
    content: &str,
) -> Result<String, String> {
    let base_path = get_user_docs_path(user_identifier);
    let doc_path = format!("{base_path}/{doc_id}.html");
    let meta_path = format!("{base_path}/{doc_id}.meta.json");

    state
        .drive
        .put_object(
            &state.bucket_name,
            &doc_path,
            content.as_bytes().to_vec(),
            Some("text/html"),
        )
        .await
        .map_err(|e| format!("Failed to save document: {e}"))?;

    let word_count = count_words(content);

    let metadata = serde_json::json!({
        "id": doc_id,
        "title": title,
        "created_at": Utc::now().to_rfc3339(),
        "updated_at": Utc::now().to_rfc3339(),
        "word_count": word_count,
        "version": 1
    });

    state
        .drive
        .put_object(
            &state.bucket_name,
            &meta_path,
            metadata.to_string().into_bytes(),
            Some("application/json"),
        )
        .await
        .map_err(|e| format!("Failed to save metadata: {e}"))?;

    Ok(doc_path)
}

pub async fn save_document(
    state: &DocState,
    user_identifier: &str,
    doc: &Document,
) -> Result<String, String> {
    save_document_to_drive(state, user_identifier, &doc.id, &doc.title, &doc.content).await
}

pub async fn load_document_from_drive(
    state: &DocState,
    user_identifier: &str,
    doc_id: &str,
) -> Result<Option<Document>, String> {
    let base_path = get_user_docs_path(user_identifier);
    let doc_path = format!("{base_path}/{doc_id}.html");
    let meta_path = format!("{base_path}/{doc_id}.meta.json");

    let content = match state
        .drive
        .get_object(&state.bucket_name, &doc_path)
        .await
    {
        Ok(bytes) => String::from_utf8(bytes).map_err(|e| e.to_string())?,
        Err(_) => return Ok(None),
    };

    let (title, created_at, updated_at) = match state
        .drive
        .get_object(&state.bucket_name, &meta_path)
        .await
    {
        Ok(bytes) => {
            let meta_str = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            let meta: serde_json::Value = serde_json::from_str(&meta_str).unwrap_or_default();
            (
                meta["title"].as_str().unwrap_or("Untitled").to_string(),
                meta["created_at"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
                meta["updated_at"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
            )
        }
        Err(_) => ("Untitled".to_string(), Utc::now(), Utc::now()),
    };

    Ok(Some(Document {
        id: doc_id.to_string(),
        title,
        content,
        owner_id: user_identifier.to_string(),
        storage_path: doc_path,
        created_at,
        updated_at,
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

pub async fn list_documents_from_drive(
    state: &DocState,
    user_identifier: &str,
) -> Result<Vec<DocumentMetadata>, String> {
    let base_path = get_user_docs_path(user_identifier);
    let prefix = format!("{base_path}/");
    let mut documents = Vec::new();

    let objects = state
        .drive
        .list_objects(&state.bucket_name, Some(&prefix))
        .await
        .map_err(|e| format!("Failed to list documents: {e}"))?;

    for key in &objects {
        if key.ends_with(".meta.json") {
            if let Ok(bytes) = state
                .drive
                .get_object(&state.bucket_name, key)
                .await
            {
                if let Ok(meta_str) = String::from_utf8(bytes) {
                    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                        let doc_meta = DocumentMetadata {
                            id: meta["id"].as_str().unwrap_or_default().to_string(),
                            title: meta["title"]
                                .as_str()
                                .unwrap_or("Untitled")
                                .to_string(),
                            owner_id: user_identifier.to_string(),
                            created_at: meta["created_at"]
                                .as_str()
                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                .map(|d| d.with_timezone(&Utc))
                                .unwrap_or_else(Utc::now),
                            updated_at: meta["updated_at"]
                                .as_str()
                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                .map(|d| d.with_timezone(&Utc))
                                .unwrap_or_else(Utc::now),
                            word_count: meta["word_count"].as_u64().unwrap_or(0) as usize,
                            storage_type: "html".to_string(),
                        };
                        documents.push(doc_meta);
                    }
                }
            }
        }
    }

    documents.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
    Ok(documents)
}

pub async fn delete_document_from_drive(
    state: &DocState,
    user_identifier: &str,
    doc_id: &str,
) -> Result<(), String> {
    let base_path = get_user_docs_path(user_identifier);

    for ext in &[".html", ".docx", ".meta.json"] {
        let path = format!("{base_path}/{doc_id}{ext}");
        let _ = state
            .drive
            .delete_object(&state.bucket_name, &path)
            .await;
    }

    remove_from_cache(doc_id).await;

    Ok(())
}
