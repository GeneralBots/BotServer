use super::*;

impl KbIndexer {
    pub(crate) async fn ensure_collection_exists(&self, collection_name: &str) -> Result<()> {
        let check_url = format!("{}/collections/{}", self.qdrant_config.url, collection_name);
        let required_dims = self.embedding_generator.get_dimensions();

        let response = self.http_client.get(&check_url).send().await?;

        if response.status().is_success() {
            // Check if the existing collection has the correct vector size
            let info_json: serde_json::Value = response.json().await?;
            let existing_dims = info_json["result"]["config"]["params"]["vectors"]["size"]
                .as_u64()
                .map(|d| d as usize);

            match existing_dims {
                Some(dims) if dims == required_dims => {
                    trace!("Collection {} already exists with correct dims ({})", collection_name, required_dims);
                    return Ok(());
                }
                Some(dims) => {
                    info!(
                        "Collection {} exists with dim={} but embedding requires dim={}. \
                        Recreating collection.",
                        collection_name, dims, required_dims
                    );
                    // Delete the stale collection so we can recreate it
                    let delete_url = format!("{}/collections/{}", self.qdrant_config.url, collection_name);
                    let del_resp = self.http_client.delete(&delete_url).send().await?;
                    if !del_resp.status().is_success() {
                        let err = del_resp.text().await.unwrap_or_default();
                        return Err(anyhow::anyhow!(
                            "Failed to delete stale collection {}: {}",
                            collection_name, err
                        ));
                    }
                    info!("Deleted stale collection {} (was dim={})", collection_name, dims);
                }
                None => {
                    // Could not read dims – recreate to be safe
                    info!("Could not read dims for collection {}, recreating", collection_name);
                    let delete_url = format!("{}/collections/{}", self.qdrant_config.url, collection_name);
                    let _ = self.http_client.delete(&delete_url).send().await;
                }
            }
        }

        info!("Creating collection {} with dim={}", collection_name, required_dims);

        let config = CollectionConfig {
            vectors: VectorConfig {
                size: required_dims,
                distance: "Cosine".to_string(),
            },
            replication_factor: 1,
            shard_number: 1,
        };

        let create_url = format!("{}/collections/{}", self.qdrant_config.url, collection_name);

        let response = self
            .http_client
            .put(&create_url)
            .json(&config)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to create collection: {}",
                error_text
            ));
        }

        self.create_collection_indexes(collection_name).await?;

        Ok(())
    }

    async fn create_collection_indexes(&self, collection_name: &str) -> Result<()> {
        let index_config = serde_json::json!({
            "hnsw_config": {
                "m": 16,
                "ef_construct": 200,
                "full_scan_threshold": 10000
            }
        });

        let index_url = format!(
            "{}/collections/{}/index",
            self.qdrant_config.url, collection_name
        );

        let response = self
            .http_client
            .put(&index_url)
            .json(&index_config)
            .send()
            .await?;

        if !response.status().is_success() {
            info!("Failed to create index, using defaults");
        }

        Ok(())
    }

    pub(super) fn create_qdrant_points(
        doc_path: &str,
        embeddings: Vec<(TextChunk, Embedding)>,
    ) -> Result<Vec<QdrantPoint>> {
        let mut points = Vec::new();

        for (chunk, embedding) in embeddings {
            let point_id = Uuid::new_v4().to_string();

            let mut payload = HashMap::new();
            payload.insert(
                "content".to_string(),
                serde_json::Value::String(chunk.content),
            );
            payload.insert(
                "document_path".to_string(),
                serde_json::Value::String(doc_path.to_string()),
            );
            payload.insert(
                "chunk_index".to_string(),
                serde_json::Value::Number(chunk.metadata.chunk_index.into()),
            );
            payload.insert(
                "total_chunks".to_string(),
                serde_json::Value::Number(chunk.metadata.total_chunks.into()),
            );
            payload.insert(
                "start_char".to_string(),
                serde_json::Value::Number(chunk.metadata.start_char.into()),
            );
            payload.insert(
                "end_char".to_string(),
                serde_json::Value::Number(chunk.metadata.end_char.into()),
            );

            if let Some(title) = chunk.metadata.document_title {
                payload.insert(
                    "document_title".to_string(),
                    serde_json::Value::String(title),
                );
            }

            points.push(QdrantPoint {
                id: point_id,
                vector: embedding.vector,
                payload,
            });
        }

        Ok(points)
    }

    pub(crate) async fn upsert_points(&self, collection_name: &str, points: Vec<QdrantPoint>) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }

        let batch_size = 100;

        for batch in points.chunks(batch_size) {
            let upsert_request = serde_json::json!({
                "points": batch
            });

            let upsert_url = format!(
                "{}/collections/{}/points?wait=true",
                self.qdrant_config.url, collection_name
            );

            let response = self
                .http_client
                .put(&upsert_url)
                .json(&upsert_request)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("Failed to upsert points: {}", error_text));
            }
        }

        debug!(
            "Upserted {} points to collection {}",
            points.len(),
            collection_name
        );

        Ok(())
    }
}
