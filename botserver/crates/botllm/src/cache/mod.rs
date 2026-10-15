pub(crate) use async_trait::async_trait;
pub(crate) use log::{debug, info, trace};
pub(crate) use redis::AsyncCommands;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::Value;
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use std::sync::Arc;
pub(crate) use std::time::{SystemTime, UNIX_EPOCH};
pub(crate) use tokio::sync::mpsc;
pub(crate) use uuid::Uuid;
pub(crate) use super::traits::LLMProvider;
pub(crate) use botcore::config::ConfigManager;
pub(crate) use botcore::shared::utils::{estimate_token_count, DbPool};

#[derive(Clone, Debug)]

pub struct CacheConfig {
    pub ttl: u64,

    pub semantic_matching: bool,

    pub similarity_threshold: f32,

    pub max_similarity_checks: usize,

    pub key_prefix: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]

pub struct CachedResponse {
    pub response: String,

    pub prompt: String,

    pub messages: Value,

    pub model: String,

    pub timestamp: u64,

    pub hit_count: u32,

    pub embedding: Option<Vec<f32>>,
}

pub struct CachedLLMProvider {
    provider: Arc<dyn LLMProvider>,

    cache: Arc<redis::Client>,

    config: CacheConfig,

    embedding_service: Option<Arc<dyn EmbeddingService>>,

    db_pool: Option<DbPool>,
}

#[async_trait]

pub trait EmbeddingService: Send + Sync {
    async fn get_embedding(
        &self,
        text: &str,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>;
    async fn compute_similarity(&self, embedding1: &[f32], embedding2: &[f32]) -> f32;
}

#[derive(Serialize, Deserialize, Clone, Debug)]

pub struct CacheStats {
    pub total_entries: usize,
    pub total_hits: u32,
    pub total_size_bytes: usize,
    pub model_distribution: std::collections::HashMap<String, u32>,
}

#[derive(Debug)]

pub struct LocalEmbeddingService {
    embedding_url: String,
    model: String,
    api_key: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
