use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

use crate::qdrant_native::QdrantClient;
use crate::re_cache::cached_re;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDocument {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: u64,
    pub bucket: String,
    pub content_text: String,
    pub content_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
    pub mime_type: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchQuery {
    pub query_text: String,
    pub bucket: Option<String>,
    pub file_type: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    pub file: FileDocument,
    pub score: f32,
    pub snippet: String,
    pub highlights: Vec<String>,
}

pub struct UserDriveVectorDB {
    user_id: Uuid,
    bot_id: Uuid,
    collection_name: String,
    db_path: PathBuf,
    client: Option<Arc<QdrantClient>>,
}

impl UserDriveVectorDB {
    pub fn new(user_id: Uuid, bot_id: Uuid, db_path: PathBuf) -> Self {
        let collection_name = format!("drive_{}_{}", bot_id, user_id);
        Self {
            user_id,
            bot_id,
            collection_name,
            db_path,
            client: None,
        }
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn bot_id(&self) -> Uuid {
        self.bot_id
    }

    pub fn collection_name(&self) -> &str {
        &self.collection_name
    }

    pub fn db_path(&self) -> &std::path::Path {
        &self.db_path
    }

    pub async fn initialize(&mut self, qdrant_url: &str) -> Result<()> {
        let client = QdrantClient::from_url(qdrant_url).build()?;
        let collections = client.list_collections().await?;
        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection_name);

        if !exists {
            client
                .create_collection(&self.collection_name, 1536, "Cosine")
                .await?;
            log::info!("Initialized vector DB collection: {}", self.collection_name);
        }

        self.client = Some(Arc::new(client));
        Ok(())
    }

    pub async fn index_file(&self, file: &FileDocument, embedding: Vec<f32>) -> Result<()> {
        if let Some(client) = self.client.as_ref() {
            let point = serde_json::json!({
                "id": file.id,
                "vector": embedding,
                "payload": serde_json::to_value(file)?
            });
            client
                .upsert_points(&self.collection_name, vec![point])
                .await?;
            log::debug!("Indexed file: {} - {}", file.id, file.file_name);
        } else {
            let file_path = self.db_path.join(format!("{}.json", file.id));
            let json = serde_json::to_string_pretty(file)?;
            fs::write(file_path, json).await?;
        }
        Ok(())
    }

    pub async fn index_files_batch(&self, files: &[(FileDocument, Vec<f32>)]) -> Result<()> {
        for (file, embedding) in files {
            self.index_file(file, embedding.clone()).await?;
        }
        Ok(())
    }

    pub async fn search(
        &self,
        query: &FileSearchQuery,
        query_embedding: Vec<f32>,
    ) -> Result<Vec<FileSearchResult>> {
        if let Some(client) = self.client.as_ref() {
            let results = client
                .search_points(
                    &self.collection_name,
                    &query_embedding,
                    query.limit,
                    None,
                )
                .await?;

            let mut search_results = Vec::new();
            for point in results {
                let payload = point
                    .get("payload")
                    .and_then(|p| p.as_object())
                    .cloned()
                    .unwrap_or_default();

                let get_str = |key: &str| -> String {
                    payload
                        .get(key)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                };

                let file = FileDocument {
                    id: get_str("id"),
                    file_path: get_str("file_path"),
                    file_name: get_str("file_name"),
                    file_type: get_str("file_type"),
                    file_size: payload
                        .get("file_size")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as u64,
                    bucket: get_str("bucket"),
                    content_text: get_str("content_text"),
                    content_summary: payload
                        .get("content_summary")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    created_at: chrono::Utc::now(),
                    modified_at: chrono::Utc::now(),
                    indexed_at: chrono::Utc::now(),
                    mime_type: payload
                        .get("mime_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    tags: vec![],
                };

                let score = point
                    .get("score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;

                let snippet = Self::create_snippet(&file.content_text, &query.query_text, 200);
                let highlights =
                    Self::extract_highlights(&file.content_text, &query.query_text, 3);

                search_results.push(FileSearchResult {
                    file,
                    score,
                    snippet,
                    highlights,
                });
            }
            Ok(search_results)
        } else {
            let mut results = Vec::new();
            let mut entries = fs::read_dir(&self.db_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = fs::read_to_string(entry.path()).await?;
                    if let Ok(file) = serde_json::from_str::<FileDocument>(&content) {
                        let query_lower = query.query_text.to_lowercase();
                        if file.file_name.to_lowercase().contains(&query_lower)
                            || file.content_text.to_lowercase().contains(&query_lower)
                        {
                            let snippet =
                                Self::create_snippet(&file.content_text, &query.query_text, 200);
                            let highlights = Self::extract_highlights(
                                &file.content_text,
                                &query.query_text,
                                3,
                            );
                            results.push(FileSearchResult {
                                file,
                                score: 1.0,
                                snippet,
                                highlights,
                            });
                        }
                    }
                    if results.len() >= query.limit {
                        break;
                    }
                }
            }
            Ok(results)
        }
    }

    fn create_snippet(content: &str, query: &str, max_length: usize) -> String {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();
        if let Some(pos) = content_lower.find(&query_lower) {
            // `pos` indexes the lowercased copy, so it is only a byte hint for
            // `content`: both edges are snapped to char boundaries, otherwise a
            // query match next to an accented character panics (#1508).
            let start = botlib::text::char_boundary_at_or_before(
                content,
                pos.saturating_sub(max_length / 2),
            );
            let end = botlib::text::char_boundary_at_or_before(
                content,
                (pos + query.len() + max_length / 2).min(content.len()),
            );
            let snippet = &content[start..end];
            if start > 0 && end < content.len() {
                format!("...{}...", snippet)
            } else if start > 0 {
                format!("...{}", snippet)
            } else if end < content.len() {
                format!("{}...", snippet)
            } else {
                snippet.to_string()
            }
        } else if content.len() > max_length {
            format!(
                "{}...",
                botlib::text::truncate_at_char_boundary(content, max_length)
            )
        } else {
            content.to_string()
        }
    }

    fn extract_highlights(content: &str, query: &str, max_highlights: usize) -> Vec<String> {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();
        let mut highlights = Vec::new();
        let mut pos = 0;
        while let Some(found_pos) = content_lower[pos..].find(&query_lower) {
            let actual_pos = pos + found_pos;
            let start = actual_pos.saturating_sub(40);
            let end = (actual_pos + query.len() + 40).min(content.len());
            highlights.push(content[start..end].to_string());
            if highlights.len() >= max_highlights {
                break;
            }
            pos = actual_pos + query.len();
        }
        highlights
    }

    pub async fn delete_file(&self, file_id: &str) -> Result<()> {
        if let Some(client) = self.client.as_ref() {
            client
                .delete_points(&self.collection_name, vec![file_id.to_string()])
                .await?;
            log::debug!("Deleted file from index: {}", file_id);
        } else {
            let file_path = self.db_path.join(format!("{}.json", file_id));
            if file_path.exists() {
                fs::remove_file(file_path).await?;
            }
        }
        Ok(())
    }

    pub async fn get_count(&self) -> Result<u64> {
        if let Some(client) = self.client.as_ref() {
            let info = client.collection_info(&self.collection_name).await?;
            Ok(info.points_count.unwrap_or(0))
        } else {
            let mut count = 0u64;
            let mut entries = fs::read_dir(&self.db_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    count += 1;
                }
            }
            Ok(count)
        }
    }

    pub async fn clear(&self) -> Result<()> {
        if let Some(client) = self.client.as_ref() {
            client.delete_collection(&self.collection_name).await?;
            client
                .create_collection(&self.collection_name, 1536, "Cosine")
                .await?;
            log::info!("Cleared drive vector collection: {}", self.collection_name);
        } else if self.db_path.exists() {
            fs::remove_dir_all(&self.db_path).await?;
            fs::create_dir_all(&self.db_path).await?;
        }
        Ok(())
    }
}

mod part2;
pub use part2::*;

#[cfg(test)]
mod snippet_tests {
    use super::UserDriveVectorDB;

    /// The window edges come from a match position in the lowercased copy, so
    /// either edge can land inside an accented character (#1508).
    #[test]
    fn snippet_around_a_multibyte_match_snaps_to_char_boundaries() {
        let content = "relatório café açúcar mascavo";
        let snippet = UserDriveVectorDB::create_snippet(content, "café", 10);
        assert!(snippet.contains("café"), "the match must survive: {snippet}");
        assert!(snippet.starts_with("..."), "{snippet}");
    }

    /// The no-match branch cut a byte budget straight out of the content.
    #[test]
    fn snippet_without_a_match_truncates_on_a_char_boundary() {
        let content = "relatório de açúcar";
        let snippet = UserDriveVectorDB::create_snippet(content, "ausente", 6);
        assert_eq!(snippet, "relat...");
    }
}
