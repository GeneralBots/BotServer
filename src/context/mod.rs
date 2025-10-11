use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::shared::SearchResult;

#[async_trait]
pub trait ContextStore: Send + Sync {
    async fn store_embedding(
        &self,
        text: &str,
        embedding: Vec<f32>,
        metadata: Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn search_similar(
        &self,
        embedding: Vec<f32>,
        limit: u32,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>>;
}

pub struct QdrantContextStore {
    vector_store: Arc<qdrant_client::client::QdrantClient>,
}

impl QdrantContextStore {
    pub fn new(vector_store: qdrant_client::client::QdrantClient) -> Self {
        Self {
            vector_store: Arc::new(vector_store),
        }
    }

    pub async fn get_conversation_context(
        &self,
        session_id: &str,
        user_id: &str,
        _limit: usize,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        let _query = format!("session_id:{} AND user_id:{}", session_id, user_id);
        Ok(vec![])
    }
}

#[async_trait]
impl ContextStore for QdrantContextStore {
    async fn store_embedding(
        &self,
        text: &str,
        _embedding: Vec<f32>,
        _metadata: Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Storing embedding for text: {}", text);
        Ok(())
    }

    async fn search_similar(
        &self,
        _embedding: Vec<f32>,
        _limit: u32,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![])
    }
}

pub struct MockContextStore;

impl MockContextStore {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ContextStore for MockContextStore {
    async fn store_embedding(
        &self,
        text: &str,
        _embedding: Vec<f32>,
        _metadata: Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Mock storing embedding for: {}", text);
        Ok(())
    }

    async fn search_similar(
        &self,
        _embedding: Vec<f32>,
        _limit: u32,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![])
    }
}
