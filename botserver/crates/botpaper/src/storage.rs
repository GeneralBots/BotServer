use crate::state::PaperState;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use super::models::{Document, DocumentMetadata};

fn get_user_papers_path(user_identifier: &str) -> String {
    let safe_id = user_identifier
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        .to_lowercase();
    format!("users/{}/papers", safe_id)
}

pub async fn save_document_to_drive(
    state: &Arc<PaperState>,
    user_identifier: &str,
    doc_id: &str,
    title: &str,
    content: &str,
    is_named: bool,
) -> Result<String, String> {
    let base_path = get_user_papers_path(user_identifier);
    let storage_type = if is_named { "named" } else { "current" };

    let (doc_path, metadata_path) = if is_named {
        let safe_title = title
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
            .to_lowercase()
            .chars()
            .take(50)
            .collect::<String>();
        (
            format!("{}/{}/{}/document.md", base_path, storage_type, safe_title),
            Some(format!(
                "{}/{}/{}/metadata.json",
                base_path, storage_type, safe_title
            )),
        )
    } else {
        (
            format!("{}/{}/{}.md", base_path, storage_type, doc_id),
            None,
        )
    };

    (state.s3_put)(&state.bucket_name, &doc_path, content.as_bytes().to_vec(), Some("text/markdown"))
        .await
        .map_err(|e| format!("Failed to save document: {}", e))?;

    if let Some(meta_path) = metadata_path {
        let metadata = serde_json::json!({
            "id": doc_id,
            "title": title,
            "created_at": Utc::now().to_rfc3339(),
            "updated_at": Utc::now().to_rfc3339(),
            "word_count": content.split_whitespace().count()
        });

        (state.s3_put)(&state.bucket_name, &meta_path, metadata.to_string().into_bytes(), Some("application/json"))
            .await
            .map_err(|e| format!("Failed to save metadata: {}", e))?;
    }

    Ok(doc_path)
}

pub async fn load_document_from_drive(
    state: &Arc<PaperState>,
    user_identifier: &str,
    doc_id: &str,
) -> Result<Option<Document>, String> {
    let base_path = get_user_papers_path(user_identifier);
    let current_path = format!("{}/current/{}.md", base_path, doc_id);

    match (state.s3_get)(&state.bucket_name, &current_path).await {
        Ok(bytes) => {
            let content = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            let title = content
                .lines()
                .next()
                .map(|l| l.trim_start_matches('#').trim())
                .unwrap_or("Untitled")
                .to_string();

            Ok(Some(Document {
                id: doc_id.to_string(),
                title,
                content,
                owner_id: user_identifier.to_string(),
                storage_path: current_path,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }))
        }
        Err(_) => Ok(None),
    }
}

pub async fn list_documents_from_drive(
    state: &Arc<PaperState>,
    user_identifier: &str,
) -> Result<Vec<DocumentMetadata>, String> {
    let base_path = get_user_papers_path(user_identifier);
    let mut documents = Vec::new();

    let current_prefix = format!("{}/current/", base_path);
    if let Ok(keys) = (state.s3_list)(&state.bucket_name, &current_prefix).await {
        for key in keys {
            if key.to_lowercase().ends_with(".md") {
                let id = key
                    .trim_start_matches(&current_prefix)
                    .trim_end_matches(".md")
                    .to_string();

                documents.push(DocumentMetadata {
                    id: id.clone(),
                    title: format!("Untitled ({})", &id[..8.min(id.len())]),
                    owner_id: user_identifier.to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    word_count: 0,
                    storage_type: "current".to_string(),
                });
            }
        }
    }

    let named_prefix = format!("{}/named/", base_path);
    if let Ok(folders) = (state.s3_list)(&state.bucket_name, &named_prefix).await {
        for folder in folders {
            let folder_name = folder
                .trim_start_matches(&named_prefix)
                .trim_end_matches('/');

            let meta_key = format!("{}metadata.json", folder);
            if let Ok(meta_bytes) = (state.s3_get)(&state.bucket_name, &meta_key).await {
                if let Ok(meta_str) = String::from_utf8(meta_bytes) {
                    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                        documents.push(DocumentMetadata {
                            id: meta["id"].as_str().unwrap_or(folder_name).to_string(),
                            title: meta["title"].as_str().unwrap_or(folder_name).to_string(),
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
                            storage_type: "named".to_string(),
                        });
                        continue;
                    }
                }
            }

            documents.push(DocumentMetadata {
                id: folder_name.to_string(),
                title: folder_name.to_string(),
                owner_id: user_identifier.to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                word_count: 0,
                storage_type: "named".to_string(),
            });
        }
    }

    documents.sort_by_key(|a| std::cmp::Reverse(a.updated_at.clone()));
    Ok(documents)
}

pub async fn delete_document_from_drive(
    state: &Arc<PaperState>,
    user_identifier: &str,
    doc_id: &str,
) -> Result<(), String> {
    let base_path = get_user_papers_path(user_identifier);
    let current_path = format!("{}/current/{}.md", base_path, doc_id);
    let _ = (state.s3_delete)(&state.bucket_name, &current_path).await;

    let named_prefix = format!("{}/named/{}/", base_path, doc_id);
    if let Ok(keys) = (state.s3_list)(&state.bucket_name, &named_prefix).await {
        for key in keys {
            let _ = (state.s3_delete)(&state.bucket_name, &key).await;
        }
    }

    Ok(())
}
