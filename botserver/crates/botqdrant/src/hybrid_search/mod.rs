pub(crate) use log::{debug, info, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::fmt::Write;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchConfig {
    pub dense_weight: f32,

    pub sparse_weight: f32,

    pub reranker_enabled: bool,

    pub reranker_model: String,

    pub max_results: usize,

    pub min_score: f32,

    pub rrf_k: u32,

    pub bm25_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub doc_id: String,

    pub content: String,

    pub source: String,

    pub score: f32,

    pub metadata: HashMap<String, String>,

    pub search_method: SearchMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchMethod {
    Dense,
    Sparse,
    Hybrid,
    Reranked,
}

pub struct BM25Index {
    doc_freq: HashMap<String, usize>,
    doc_count: usize,
    avg_doc_len: f32,
    doc_lengths: HashMap<String, usize>,
    term_freqs: HashMap<String, HashMap<String, usize>>,
    doc_sources: HashMap<String, String>,
    k1: f32,
    b: f32,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25Stats {
    pub doc_count: usize,
    pub unique_terms: usize,
    pub avg_doc_len: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
struct DocumentEntry {
    pub content: String,
    pub source: String,
    pub metadata: HashMap<String, String>,
}

pub struct HybridSearchEngine {
    bm25_index: BM25Index,

    documents: HashMap<String, DocumentEntry>,

    config: HybridSearchConfig,

    qdrant_url: String,

    collection_name: String,
}

#[derive(Debug, Clone)]
pub struct HybridSearchStats {
    pub total_documents: usize,
    pub bm25_doc_count: usize,
    pub unique_terms: usize,
    pub avg_doc_len: f32,
    pub bm25_enabled: bool,
    pub config: HybridSearchConfig,
}

pub struct QueryDecomposer {
    llm_endpoint: String,
    api_key: String,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

use part5::*;
