pub(crate) use anyhow::{Context, Result};
pub(crate) use log::{info, trace, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use crate::shared::memory_monitor::{log_jemalloc_stats, MemoryStats};
pub(crate) use std::sync::atomic::{AtomicBool, Ordering};

static EMBEDDING_SERVER_READY: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub embedding_url: String,
    pub embedding_model: String,
    pub embedding_key: Option<String>,
    pub dimensions: usize,
    pub batch_size: usize,
    pub timeout_seconds: u64,
    pub max_concurrent_requests: usize,
    pub connect_timeout_seconds: u64,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

// OpenAI/Claude/OpenAI-compatible format
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
    model: String,
    usage: Option<EmbeddingUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

// llama.cpp format
#[derive(Debug, Deserialize)]
struct LlamaCppEmbeddingItem {
    embedding: Vec<Vec<f32>>,
}

// Hugging Face/SentenceTransformers format (simple array)
type HuggingFaceEmbeddingResponse = Vec<Vec<f32>>;

// Scaleway/OpenAI-compatible format (object with data array)
#[derive(Debug, Deserialize)]
struct ScalewayEmbeddingResponse {
    data: Vec<ScalewayEmbeddingData>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<EmbeddingUsage>,
}

#[derive(Debug, Deserialize)]
struct ScalewayEmbeddingData {
    embedding: Vec<f32>,
}

// Generic embedding service format (object with embeddings key)
#[derive(Debug, Deserialize)]
struct GenericEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<EmbeddingUsage>,
}

// Cloudflare AI Workers format
#[derive(Debug, Serialize)]
struct CloudflareEmbeddingRequest {
    text: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CloudflareEmbeddingResponse {
    result: CloudflareResult,
    success: bool,
    #[serde(default)]
    errors: Vec<CloudflareError>,
}

#[derive(Debug, Deserialize)]
struct CloudflareResult {
    data: Vec<Vec<f32>>,
    #[serde(default)]
    meta: Option<CloudflareMeta>,
}

#[derive(Debug, Deserialize)]
struct CloudflareMeta {
    #[serde(default)]
    cost_metric_value_1: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
    #[serde(default)]
    code: i32,
    #[serde(default)]
    message: String,
}

// Universal response wrapper - tries formats in order of likelihood
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EmbeddingResponse {
    Scaleway(ScalewayEmbeddingResponse),       // Scaleway/OpenAI-compatible format
    OpenAI(OpenAIEmbeddingResponse),           // Most common: OpenAI, Claude, etc.
    LlamaCpp(Vec<LlamaCppEmbeddingItem>),      // llama.cpp server
    HuggingFace(HuggingFaceEmbeddingResponse), // Simple array format
    Generic(GenericEmbeddingResponse),         // Generic services
    Cloudflare(CloudflareEmbeddingResponse),   // Cloudflare AI Workers
}

#[derive(Debug, Deserialize)]
struct EmbeddingUsage {
    #[serde(default)]
    total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub dimensions: usize,
    pub model: String,
    pub tokens_used: Option<usize>,
}

pub struct KbEmbeddingGenerator {
    config: std::sync::RwLock<EmbeddingConfig>,
    client: Client,
    semaphore: Arc<Semaphore>,
}

pub struct EmbeddingGenerator {
    kb_generator: KbEmbeddingGenerator,
}

pub struct EmailEmbeddingGenerator {
    generator: EmbeddingGenerator,
}

pub trait EmailLike {
    fn subject(&self) -> &str;
    fn from(&self) -> &str;
    fn to(&self) -> &str;
    fn body(&self) -> &str;
}

#[derive(Debug)]
pub struct SimpleEmail {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub body: String,
}

use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use crate::shared::DbPool;
use super::document_processor::TextChunk;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;

pub use part1::*;
use part2::*;
