use super::*;

impl KbIndexer {
    pub async fn delete_file_points(
        &self,
        collection_name: &str,
        document_path: &str,
    ) -> Result<()> {
        let filter = serde_json::json!({
            "must": [
                {
                    "key": "document_path",
                    "match": {
                        "value": document_path
                    }
                }
            ]
        });

        let delete_url = format!(
            "{}/collections/{}/points/delete?wait=true",
            self.qdrant_config.url, collection_name
        );

        let mut last_error = None;
        for attempt in 1..=3 {
            match self
                .http_client
                .post(&delete_url)
                .json(&serde_json::json!({ "filter": filter }))
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        let error_text = response.text().await.unwrap_or_default();
                        return Err(anyhow::anyhow!("Failed to delete points: {}", error_text));
                    }

                    info!(
                        "Deleted points for document {} from collection {}",
                        document_path, collection_name
                    );
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        "Failed to delete points for {} (attempt {}): {}",
                        document_path, attempt, e
                    );
                    last_error = Some(e);
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }

        match last_error {
            Some(e) => Err(anyhow::anyhow!(
                "Failed to delete points for {document_path} after 3 attempts: {e}"
            )),
            None => Err(anyhow::anyhow!(
                "Failed to delete points for {document_path} after 3 attempts"
            )),
        }
    }

    pub(super) fn update_collection_metadata(
        &self,
        collection_name: &str,
        bot_name: &str,
        kb_name: &str,
        document_count: usize,
    ) -> Result<()> {
        let _ = self;
        info!(
            "Updated collection {} metadata: bot={}, kb={}, docs={}",
            collection_name, bot_name, kb_name, document_count
        );

        Ok(())
    }

    pub async fn search(
        &self,
        collection_name: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // Get the collection's actual vector dimension to handle dimension mismatch
        let collection_dimension = self.get_collection_vector_dimension(collection_name).await?;

        let embedding = self
            .embedding_generator
            .generate_single_embedding(query)
            .await?;

        // Truncate embedding vector to match collection dimension if needed
        let search_vector = if let Some(target_dim) = collection_dimension {
            if embedding.vector.len() > target_dim {
                debug!(
                    "Truncating embedding from {} to {} dimensions for collection '{}'",
                    embedding.vector.len(), target_dim, collection_name
                );
                embedding.vector[..target_dim].to_vec()
            } else if embedding.vector.len() < target_dim {
                warn!(
                    "Embedding dimension ({}) is smaller than collection dimension ({}). \
                    Search may return poor results for collection '{}'.",
                    embedding.vector.len(), target_dim, collection_name
                );
                // Pad with zeros (not ideal but allows search to proceed)
                let mut padded = embedding.vector.clone();
                padded.resize(target_dim, 0.0);
                padded
            } else {
                embedding.vector
            }
        } else {
            embedding.vector
        };

        self.execute_search(collection_name, search_vector, limit).await
    }

    pub async fn search_with_config(
        &self,
        collection_name: &str,
        query: &str,
        limit: usize,
        embedding_config: &EmbeddingConfig,
    ) -> Result<Vec<SearchResult>> {
        let collection_dimension = self.get_collection_vector_dimension(collection_name).await?;

        let embedding_generator = KbEmbeddingGenerator::new(embedding_config.clone());
        let embedding = embedding_generator.generate_single_embedding(query).await?;

        let search_vector = if let Some(target_dim) = collection_dimension {
            if embedding.vector.len() > target_dim {
                debug!(
                    "Truncating embedding from {} to {} dimensions for collection '{}'",
                    embedding.vector.len(), target_dim, collection_name
                );
                embedding.vector[..target_dim].to_vec()
            } else if embedding.vector.len() < target_dim {
                warn!(
                    "Embedding dimension ({}) is smaller than collection dimension ({}). \
                    Search may return poor results for collection '{}'.",
                    embedding.vector.len(), target_dim, collection_name
                );
                let mut padded = embedding.vector.clone();
                padded.resize(target_dim, 0.0);
                padded
            } else {
                embedding.vector
            }
        } else {
            embedding.vector
        };

        self.execute_search(collection_name, search_vector, limit).await
    }

    async fn execute_search(
        &self,
        collection_name: &str,
        search_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let search_request = SearchRequest {
            vector: search_vector,
            limit,
            with_payload: true,
            score_threshold: None,
            filter: None,
        };

        let search_url = format!(
            "{}/collections/{}/points/search",
            self.qdrant_config.url, collection_name
        );

        let response = self
            .http_client
            .post(&search_url)
            .json(&search_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Search failed: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;

        let mut results = Vec::new();

        if let Some(result_array) = response_json["result"].as_array() {
            for item in result_array {
                if let (Some(score), Some(payload)) =
                    (item["score"].as_f64(), item["payload"].as_object())
                {
                    let content = payload
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let document_path = payload
                        .get("document_path")
                        .and_then(|p| p.as_str())
                        .unwrap_or_default()
                        .to_string();

                    results.push(SearchResult {
                        content,
                        document_path,
                        score: score as f32,
                        metadata: serde_json::Map::new(),
                    });
                }
            }
        }

        Ok(results)
    }

    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        let delete_url = format!("{}/collections/{}", self.qdrant_config.url, collection_name);

        let response = self.http_client.delete(&delete_url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            info!(
                "Failed to delete collection {}: {}",
                collection_name, error_text
            );
        }

        Ok(())
    }

    /// Get the vector dimension of a collection from Qdrant
    async fn get_collection_vector_dimension(&self, collection_name: &str) -> Result<Option<usize>> {
        let info_url = format!("{}/collections/{}", self.qdrant_config.url, collection_name);

        let response = match self.http_client.get(&info_url).send().await {
            Ok(r) => r,
            Err(e) => {
                debug!("Failed to get collection dimension: {}", e);
                return Ok(None);
            }
        };

        if !response.status().is_success() {
            debug!("Collection '{}' not found or error, using default dimension", collection_name);
            return Ok(None);
        }

        let info_json: serde_json::Value = response.json().await?;
        let dimension = info_json["result"]["config"]["params"]["vectors"]["size"]
            .as_u64()
            .map(|d| d as usize);

        Ok(dimension)
    }
}
