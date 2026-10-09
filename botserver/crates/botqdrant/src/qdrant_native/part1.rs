use super::*;

impl QdrantFromUrl {
    pub fn build(self) -> Result<QdrantClient> {
        Ok(QdrantClient::new(&self.url))
    }
}
impl QdrantClient {
    /// Create new Qdrant client from URL (returns a builder)
    pub fn from_url(url: &str) -> QdrantFromUrl {
        QdrantFromUrl { url: url.to_string() }
    }

    /// Create new Qdrant client directly from URL
    pub fn new(url: &str) -> Self {
        let client = Client::builder()
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            url: url.trim_end_matches('/').to_string(),
        }
    }

    /// Get full URL for collections endpoint
    pub(super) fn collections_url(&self) -> String {
        format!("{}/collections", self.url)
    }

    /// Create collection
    pub async fn create_collection(
        &self,
        name: &str,
        vector_size: u64,
        distance: &str,
    ) -> Result<()> {
        let url = self.collections_url();

        debug!("Creating collection: {} at {}", name, url);
        
        let response = self
            .client
            .post(&url)
            .json(&json!({
                "name": name,
                "vectors": {
                    "size": vector_size,
                    "distance": distance
                }
            }))
            .send()
            .await
            .context("Failed to send create collection request")?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        
        if status.is_success() {
            info!("Collection '{}' created successfully", name);
            Ok(())
        } else {
            log::error!("Failed to create collection '{}': {} - {}", name, status, text);
            Err(anyhow::anyhow!("HTTP {}: {}", status, text))
        }
    }

    /// List all collections
    pub async fn list_collections(&self) -> Result<CollectionsResponse> {
        let url = self.collections_url();
        
        debug!("Listing collections at {}", url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send list collections request")?;
        
        let status = response.status();
        if status.is_success() {
            let result: CollectionsResponse = response.json().await
                .context("Failed to parse collections response")?;
            debug!("Found {} collections", result.collections.len());
            Ok(result)
        } else {
            let text = response.text().await.unwrap_or_default();
            log::error!("Failed to list collections: {} - {}", status, text);
            Err(anyhow::anyhow!("HTTP {}: {}", status, text))
        }
    }

    /// Check if collection exists
    pub async fn collection_exists(&self, name: &str) -> Result<bool> {
        let url = format!("{}/{}", self.collections_url(), name);

        debug!("Checking collection: {} at {}", name, url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send collection exists request")?;

        Ok(response.status().is_success())
    }

    /// Get collection info
    pub async fn collection_info(&self, name: &str) -> Result<CollectionInfoResponse> {
        let url = format!("{}/{}", self.collections_url(), name);

        debug!("Getting collection info: {} at {}", name, url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send collection info request")?;

        let status = response.status();
        if status.is_success() {
            let result: CollectionInfoResponse = response.json().await
                .context("Failed to parse collection info response")?;
            Ok(result)
        } else {
            let text = response.text().await.unwrap_or_default();
            log::error!("Failed to get collection info: {} - {}", status, text);
            Err(anyhow::anyhow!("HTTP {}: {}", status, text))
        }
    }

    /// Delete collection
    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        let url = format!("{}/{}", self.collections_url(), name);

        debug!("Deleting collection: {} at {}", name, url);

        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .context("Failed to send delete collection request")?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if status.is_success() {
            info!("Collection '{}' deleted successfully", name);
            Ok(())
        } else {
            log::error!("Failed to delete collection '{}': {} - {}", name, status, text);
            Err(anyhow::anyhow!("HTTP {}: {}", status, text))
        }
    }

    /// Upsert points into collection
    pub async fn upsert_points(
        &self,
        collection_name: &str,
        points: Vec<Value>,
    ) -> Result<()> {
        let url = format!("{}/{}/upsert", self.collections_url(), collection_name);
        let body = json!({
            "points": points
        });

        debug!("Upserting {} points to {}", points.len(), collection_name);
        
        let response = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send upsert request")?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        
        if status.is_success() {
            debug!("Successfully upserted {} points", points.len());
            Ok(())
        } else {
            log::error!("Failed to upsert points: {} - {}", status, text);
            Err(anyhow::anyhow!("HTTP {}: {}", status, text))
        }
    }

    /// Search points in collection
    pub async fn search_points(
        &self,
        collection_name: &str,
        vector: &[f32],
        limit: usize,
        filter: Option<Value>,
    ) -> Result<Vec<Value>> {
        let url = format!("{}/{}/search", self.collections_url(), collection_name);
        let mut body = json!({
            "vector": vector,
            "limit": limit
        });

        if let Some(f) = filter {
            body["filter"] = f;
        }

        debug!("Searching {} points in {}", limit, collection_name);
        
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send search request")?;

        let status = response.status();
        if status.is_success() {
            let result: Value = response.json().await.context("Failed to parse search response")?;
            let points = result["result"]
                .as_array().cloned()
                .unwrap_or_default();
            debug!("Found {} search results", points.len());
            Ok(points)
        } else {
            let text = response.text().await.unwrap_or_default();
            log::error!("Failed to search points: {} - {}", status, text);
            Err(anyhow::anyhow!("HTTP {}: {}", status, text))
        }
    }

    /// Delete points from collection
    pub async fn delete_points(
        &self,
        collection_name: &str,
        point_ids: Vec<String>,
    ) -> Result<()> {
        let url = format!("{}/{}/delete", self.collections_url(), collection_name);
        let body = json!({
            "points": point_ids
        });

        debug!("Deleting {} points from {}", point_ids.len(), collection_name);
        
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send delete request")?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        
        if status.is_success() {
            info!("Successfully deleted {} points", point_ids.len());
            Ok(())
        } else {
            log::error!("Failed to delete points: {} - {}", status, text);
            Err(anyhow::anyhow!("HTTP {}: {}", status, text))
        }
    }
}
