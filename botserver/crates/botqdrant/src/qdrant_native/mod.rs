/// Qdrant HTTP Client - Native implementation without qdrant-client crate
/// Uses reqwest for HTTP communication with Qdrant REST API
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use reqwest::Client;
use log::{debug, info};

/// Qdrant client using native HTTP (old name for backward compatibility)
pub type Qdrant = QdrantClient;

/// Qdrant client using native HTTP
#[derive(Clone)]
pub struct QdrantClient {
    client: Client,
    url: String,
}

/// Builder trait for Qdrant client
pub trait Build {
    fn build(self) -> Result<QdrantClient>;
}

/// URL builder for Qdrant
pub struct QdrantFromUrl {
    url: String,
}

/// Response for list collections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionsResponse {
    pub collections: Vec<CollectionInfo>,
}

/// Collection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub name: String,
}

/// Collection info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfoResponse {
    #[serde(default)]
    pub points_count: Option<u64>,
}

/// Builder for creating collections
pub struct CreateCollectionBuilder {
    name: String,
    vector_size: u64,
    distance: String,
}

/// Builder for vector parameters
pub struct VectorParamsBuilder {
    size: u64,
    distance: String,
}

/// Distance metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Distance {
    #[default]
    Cosine,
    Euclid,
    Dot,
    Manhattan,
}

/// Point structure for Qdrant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointStruct {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: serde_json::Map<String, Value>,
}

/// Filter for search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub must: Vec<Condition>,
}

/// Search condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    #[serde(flatten)]
    pub condition_type: ConditionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionType {
    Field { key: String, value: Value },
}

/// Builder for upsert points
pub struct UpsertPointsBuilder {
    collection_name: String,
    points: Vec<Value>,
}

/// Builder for search points
pub struct SearchPointsBuilder {
    collection_name: String,
    vector: Vec<f32>,
    limit: usize,
    filter: Option<Value>,
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub result: Vec<Value>,
}

/// Payload type alias
pub type Payload = serde_json::Map<String, serde_json::Value>;

/// Point ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointId {
    pub id: String,
}

/// Vector parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorParams {
    pub size: u64,
    pub distance: Distance,
}

/// Delete points builder
pub struct DeletePointsBuilder {
    collection_name: String,
    points: Vec<PointId>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;
mod part11;
