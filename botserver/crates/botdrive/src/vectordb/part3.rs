use super::*;

impl UserDriveVectorDB {
#[cfg(not(feature = "vectordb"))]
pub async fn search(
    &self,
    query: &FileSearchQuery,
    _query_embedding: Vec<f32>,
) -> Result<Vec<FileSearchResult>> {
    let mut results = Vec::new();
    let mut entries = fs::read_dir(&self.db_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
            let path = entry.path();
            let content = tokio::task::spawn_blocking(move || {
                crate::stream_processor::read_file_streaming(&path)
            })
            .await?
            .map_err(|e| anyhow::anyhow!(e))?;
            if let Ok(file) = serde_json::from_str::<FileDocument>(&content) {
                if let Some(bucket) = &query.bucket {
                    if &file.bucket != bucket {
                        continue;
                    }
                }

                if let Some(file_type) = &query.file_type {
                    if &file.file_type != file_type {
                        continue;
                    }
                }

                let query_lower = query.query_text.to_lowercase();
                if file.file_name.to_lowercase().contains(&query_lower)
                    || file.content_text.to_lowercase().contains(&query_lower)
                    || file
                        .content_summary
                        .as_ref()
                        .is_some_and(|s| s.to_lowercase().contains(&query_lower))
                {
                    let snippet =
                        Self::create_snippet(&file.content_text, &query.query_text, 200);
                    let highlights =
                        Self::extract_highlights(&file.content_text, &query.query_text, 3);

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

pub(super) fn create_snippet(content: &str, query: &str, max_length: usize) -> String {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    if let Some(pos) = content_lower.find(&query_lower) {
        let start = pos.saturating_sub(max_length / 2);
        let end = (pos + query.len() + max_length / 2).min(content.len());
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
        format!("{}...", &content[..max_length])
    } else {
        content.to_string()
    }
}

pub(super) fn extract_highlights(content: &str, query: &str, max_highlights: usize) -> Vec<String> {
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

#[cfg(feature = "vectordb")]
pub async fn delete_file(&self, file_id: &str) -> Result<()> {
    let client = self
        .client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Vector DB not initialized"))?;

        let builder = botqdrant::DeletePointsBuilder::new(&self.collection_name).points(
            vec![botqdrant::PointId::from(file_id.to_string())],
        );
        builder.build(client).await?;

    log::debug!("Deleted file from index: {}", file_id);
    Ok(())
}

#[cfg(not(feature = "vectordb"))]
pub async fn delete_file(&self, file_id: &str) -> Result<()> {
    let file_path = self.db_path.join(format!("{}.json", file_id));
    if file_path.exists() {
        fs::remove_file(file_path).await?;
    }
    Ok(())
}

#[cfg(feature = "vectordb")]
    pub async fn get_count(&self) -> Result<u64> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Vector DB not initialized"))?;

        let info = client.collection_info(&self.collection_name).await?;

        Ok(info.points_count.unwrap_or(0))
    }

#[cfg(not(feature = "vectordb"))]
pub async fn get_count(&self) -> Result<u64> {
    let mut count = 0;
    let mut entries = fs::read_dir(&self.db_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
            count += 1;
        }
    }

    Ok(count)
}

pub async fn update_file_metadata(&self, file_id: &str, tags: Vec<String>) -> Result<()> {
    #[cfg(not(feature = "vectordb"))]
    {
        let file_path = self.db_path.join(format!("{}.json", file_id));
        if file_path.exists() {
            let path = file_path.clone();
            let content = tokio::task::spawn_blocking(move || {
                crate::stream_processor::read_file_streaming(&path)
            })
            .await?
            .map_err(|e| anyhow::anyhow!(e))?;
            let mut file: FileDocument = serde_json::from_str(&content)?;
            file.tags = tags;
            let json = serde_json::to_string_pretty(&file)?;
            fs::write(file_path, json).await?;
        }
    }

    #[cfg(feature = "vectordb")]
    {
        let _ = (file_id, tags);
        log::warn!("Metadata update not yet implemented for Qdrant backend");
    }

    Ok(())
}

#[cfg(feature = "vectordb")]
    pub async fn clear(&self) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Vector DB not initialized"))?;

        client.delete_collection(&self.collection_name).await?;

        client.create_collection(&self.collection_name, 1536, "Cosine").await?;

        log::info!("Cleared drive vector collection: {}", self.collection_name);
        Ok(())
    }

#[cfg(not(feature = "vectordb"))]
pub async fn clear(&self) -> Result<()> {
    if self.db_path.exists() {
        fs::remove_dir_all(&self.db_path).await?;
        fs::create_dir_all(&self.db_path).await?;
    }
    Ok(())
}
}
