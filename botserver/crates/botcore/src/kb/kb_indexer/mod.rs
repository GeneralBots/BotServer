pub(crate) use anyhow::Result;
pub(crate) use log::{debug, info, trace, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use crate::shared::memory_monitor::{log_jemalloc_stats, MemoryStats};
pub(crate) use crate::shared::utils::{create_tls_client, DbPool, get_vectordb_config_from_value};
pub(crate) use super::document_processor::{DocumentProcessor, TextChunk};
pub(crate) use super::embedding_generator::{is_embedding_server_ready, Embedding, EmbeddingConfig, KbEmbeddingGenerator};

#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub timeout_secs: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QdrantPoint {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CollectionConfig {
    pub vectors: VectorConfig,
    pub replication_factor: u32,
    pub shard_number: u32,
}

#[derive(Debug, Serialize)]
pub struct VectorConfig {
    pub size: usize,
    pub distance: String,
}

#[derive(Debug, Serialize)]
pub struct SearchRequest {
    pub vector: Vec<f32>,
    pub limit: usize,
    pub with_payload: bool,
    pub score_threshold: Option<f32>,
    pub filter: Option<serde_json::Value>,
}

pub struct KbIndexer {
    document_processor: DocumentProcessor,
    embedding_generator: KbEmbeddingGenerator,
    qdrant_config: QdrantConfig,
    http_client: reqwest::Client,
    db_pool: Option<DbPool>,
}

pub async fn index_single_file(
    &self,
    bot_id: Uuid,
    bot_name: &str,
    kb_name: &str,
    file_path: &Path,
) -> Result<IndexingResult> {
    self.index_single_file_with_id(bot_id, bot_name, kb_name, file_path, None).await
}

pub async fn index_single_file_with_id(
    &self,
    bot_id: Uuid,
    bot_name: &str,
    kb_name: &str,
    file_path: &Path,
    document_id: Option<&str>,
    ) -> Result<IndexingResult> {
        // Reload embedding config from database to detect config.csv changes
        if let Some(ref pool) = self.db_pool {
            self.embedding_generator.reload_config(pool, &bot_id);
        }

        if !is_embedding_server_ready()
            && !self.embedding_generator.wait_for_server(30).await
        {
                return Err(anyhow::anyhow!(
                    "Embedding server not available. Cannot index file."
                ));
        }

    if !self.check_qdrant_health().await.unwrap_or(false) {
        return Err(anyhow::anyhow!(
            "Qdrant vector database is not available."
        ));
    }

    let bot_id_short = bot_id.to_string().chars().take(8).collect::<String>();
    let collection_name = format!("{}_{}_{}", bot_name, bot_id_short, kb_name);

    self.ensure_collection_exists(&collection_name).await?;

    let doc_path = document_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| file_path.to_string_lossy().to_string());

    info!(
        "Indexing single file: {} (id: {}) into collection {}",
        file_path.display(), doc_path, collection_name
    );

    if let Err(e) = self.delete_file_points(&collection_name, &doc_path).await {
        warn!("Failed to delete old points for {} before reindex: {}", doc_path, e);
    }

    let chunks = self.document_processor.process_document(file_path).await?;

    if chunks.is_empty() {
        info!("No chunks extracted from file: {}", file_path.display());
        return Ok(IndexingResult {
            collection_name,
            documents_processed: 0,
            chunks_indexed: 0,
        });
    }

    let embeddings = self
        .embedding_generator
        .generate_embeddings(&chunks)
        .await?;

    let points = Self::create_qdrant_points(&doc_path, embeddings)?;
    self.upsert_points(&collection_name, points).await?;

        self.update_collection_metadata(&collection_name, bot_name, kb_name, chunks.len())?;

        info!(
            "Indexed {} chunks from {} into collection {}",
            chunks.len(),
            file_path.display(),
            collection_name
        );

        Ok(IndexingResult {
            collection_name,
            documents_processed: 1,
            chunks_indexed: chunks.len(),
        })
    }

#[derive(Debug, Clone)]
pub struct CollectionInfo {
    pub name: String,
    pub points_count: usize,
    pub vectors_count: usize,
    pub indexed_vectors_count: usize,
    pub segments_count: usize,
    pub status: String,
}

#[derive(Debug)]
pub struct IndexingResult {
    pub collection_name: String,
    pub documents_processed: usize,
    pub chunks_indexed: usize,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub content: String,
    pub document_path: String,
    pub score: f32,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct KbFolderMonitor {
    indexer: KbIndexer,
    work_root: PathBuf,
}

use std::collections::HashMap;
use uuid::Uuid;
use crate::config::ConfigManager;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
