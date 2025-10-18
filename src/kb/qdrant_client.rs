use crate::shared::state::AppState;
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct QdrantPoint {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCollectionRequest {
    pub vectors: VectorParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorParams {
    pub size: usize,
    pub distance: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertRequest {
    pub points: Vec<QdrantPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub vector: Vec<f32>,
    pub limit: usize,
    pub with_payload: bool,
    pub with_vector: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub result: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub payload: Option<serde_json::Value>,
    pub vector: Option<Vec<f32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub status: String,
}

pub struct QdrantClient {
    base_url: String,
    client: Client,
}

impl QdrantClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }

    /// Check if collection exists
    pub async fn collection_exists(
        &self,
        collection_name: &str,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);

        debug!("Checking if collection exists: {}", collection_name);

        let response = self.client.get(&url).send().await?;

        Ok(response.status().is_success())
    }

    /// Create a new collection
    pub async fn create_collection(
        &self,
        collection_name: &str,
        vector_size: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);

        info!(
            "Creating Qdrant collection: {} with vector size {}",
            collection_name, vector_size
        );

        let request = CreateCollectionRequest {
            vectors: VectorParams {
                size: vector_size,
                distance: "Cosine".to_string(),
            },
        };

        let response = self.client.put(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Failed to create collection: {}", error_text);
            return Err(format!("Failed to create collection: {}", error_text).into());
        }

        info!("Collection created successfully: {}", collection_name);
        Ok(())
    }

    /// Delete a collection
    pub async fn delete_collection(
        &self,
        collection_name: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let url = format!("{}/collections/{}", self.base_url, collection_name);

        info!("Deleting Qdrant collection: {}", collection_name);

        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Failed to delete collection: {}", error_text);
            return Err(format!("Failed to delete collection: {}", error_text).into());
        }

        info!("Collection deleted successfully: {}", collection_name);
        Ok(())
    }

    /// Upsert points (documents) into collection
    pub async fn upsert_points(
        &self,
        collection_name: &str,
        points: Vec<QdrantPoint>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let url = format!("{}/collections/{}/points", self.base_url, collection_name);

        debug!(
            "Upserting {} points to collection: {}",
            points.len(),
            collection_name
        );

        let request = UpsertRequest { points };

        let response = self.client.put(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Failed to upsert points: {}", error_text);
            return Err(format!("Failed to upsert points: {}", error_text).into());
        }

        debug!("Points upserted successfully");
        Ok(())
    }

    /// Search for similar vectors
    pub async fn search(
        &self,
        collection_name: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error + Send + Sync>> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.base_url, collection_name
        );

        debug!(
            "Searching in collection: {} with limit {}",
            collection_name, limit
        );

        let request = SearchRequest {
            vector: query_vector,
            limit,
            with_payload: true,
            with_vector: false,
        };

        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Search failed: {}", error_text);
            return Err(format!("Search failed: {}", error_text).into());
        }

        let search_response: SearchResponse = response.json().await?;

        debug!("Search returned {} results", search_response.result.len());

        Ok(search_response.result)
    }

    /// Delete points by filter
    pub async fn delete_points(
        &self,
        collection_name: &str,
        point_ids: Vec<String>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let url = format!(
            "{}/collections/{}/points/delete",
            self.base_url, collection_name
        );

        debug!(
            "Deleting {} points from collection: {}",
            point_ids.len(),
            collection_name
        );

        let request = serde_json::json!({
            "points": point_ids
        });

        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Failed to delete points: {}", error_text);
            return Err(format!("Failed to delete points: {}", error_text).into());
        }

        debug!("Points deleted successfully");
        Ok(())
    }
}

/// Get Qdrant client from app state
pub fn get_qdrant_client(_state: &AppState) -> Result<QdrantClient, Box<dyn Error + Send + Sync>> {
    let qdrant_url =
        std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());

    Ok(QdrantClient::new(qdrant_url))
}

/// Ensure a collection exists, create if not
pub async fn ensure_collection_exists(
    state: &AppState,
    collection_name: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = get_qdrant_client(state)?;

    if !client.collection_exists(collection_name).await? {
        info!("Collection {} does not exist, creating...", collection_name);
        // Default vector size for embeddings (adjust based on your embedding model)
        let vector_size = 1536; // OpenAI ada-002 size
        client
            .create_collection(collection_name, vector_size)
            .await?;
    } else {
        debug!("Collection {} already exists", collection_name);
    }

    Ok(())
}

/// Search documents in a collection
pub async fn search_documents(
    state: &AppState,
    collection_name: &str,
    query_embedding: Vec<f32>,
    limit: usize,
) -> Result<Vec<SearchResult>, Box<dyn Error + Send + Sync>> {
    let client = get_qdrant_client(state)?;
    client.search(collection_name, query_embedding, limit).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qdrant_client_creation() {
        let client = QdrantClient::new("http://localhost:6333".to_string());
        assert_eq!(client.base_url, "http://localhost:6333");
    }
}
