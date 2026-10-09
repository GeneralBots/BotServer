pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum InsightFaceModel {
    #[serde(rename = "buffalo_l")]
    #[default]
    BuffaloL,
    #[serde(rename = "buffalo_m")]
    BuffaloM,
    #[serde(rename = "buffalo_s")]
    BuffaloS,
    #[serde(rename = "buffalo_sc")]
    BuffaloSc,
    #[serde(rename = "antelopev2")]
    Antelopev2,
    #[serde(rename = "glintr100")]
    Glintr100,
    #[serde(rename = "w600k_r50")]
    W600kR50,
    #[serde(rename = "w600k_mbf")]
    W600kMbf,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightFaceConfig {
    pub model: InsightFaceModel,
    pub det_size: (u32, u32),
    pub det_thresh: f32,
    pub ctx_id: i32,
    pub model_path: Option<PathBuf>,
    pub enable_recognition: bool,
    pub enable_age_gender: bool,
    pub enable_landmarks: bool,
    pub enable_pose: bool,
    pub max_faces: Option<u32>,
    pub gpu_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedFace {
    pub id: Uuid,
    pub bbox: BoundingBox,
    pub detection_score: f32,
    pub landmarks: Option<FaceLandmarks>,
    pub embedding: Option<Vec<f32>>,
    pub age: Option<u8>,
    pub gender: Option<Gender>,
    pub pose: Option<FacePose>,
    pub quality: Option<FaceQuality>,
    pub attributes: HashMap<String, AttributeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceLandmarks {
    pub points_2d: Vec<Point2D>,
    pub points_3d: Option<Vec<Point3D>>,
    pub landmark_type: LandmarkType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LandmarkType {
    FivePoint,
    SixtyEightPoint,
    OneFiveSixPoint,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacePose {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceQuality {
    pub overall_score: f32,
    pub brightness: f32,
    pub sharpness: f32,
    pub occlusion: f32,
    pub blur: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Float(f32),
    Int(i32),
    Bool(bool),
    FloatArray(Vec<f32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceRecognitionResult {
    pub query_face_id: Uuid,
    pub matches: Vec<FaceMatch>,
    pub search_time_ms: u64,
    pub total_candidates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceMatch {
    pub face_id: Uuid,
    pub identity_id: Option<String>,
    pub similarity: f32,
    pub distance: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceVerificationResult {
    pub face1_id: Uuid,
    pub face2_id: Uuid,
    pub is_same_person: bool,
    pub similarity: f32,
    pub distance: f32,
    pub threshold_used: f32,
    pub verification_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceIndex {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub face_count: u32,
    pub embedding_dimension: u32,
    pub distance_metric: DistanceMetric,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DistanceMetric {
    #[default]
    Cosine,
    Euclidean,
    DotProduct,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFace {
    pub face_id: Uuid,
    pub index_id: Uuid,
    pub identity_id: Option<String>,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRequest {
    pub image_data: Vec<u8>,
    pub image_format: ImageFormat,
    pub options: DetectionOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Bmp,
    Raw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionOptions {
    pub detect_landmarks: bool,
    pub extract_embedding: bool,
    pub detect_age_gender: bool,
    pub detect_pose: bool,
    pub compute_quality: bool,
    pub min_face_size: Option<u32>,
    pub max_faces: Option<u32>,
    pub detection_threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResponse {
    pub request_id: Uuid,
    pub faces: Vec<DetectedFace>,
    pub image_width: u32,
    pub image_height: u32,
    pub processing_time_ms: u64,
    pub model_used: InsightFaceModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceCluster {
    pub cluster_id: Uuid,
    pub identity_id: Option<String>,
    pub face_ids: Vec<Uuid>,
    pub centroid: Vec<f32>,
    pub quality_score: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringConfig {
    pub min_cluster_size: u32,
    pub distance_threshold: f32,
    pub algorithm: ClusteringAlgorithm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusteringAlgorithm {
    DbScan,
    HierarchicalClustering,
    ChineseWhispers,
}

pub struct InsightFaceService {
    config: Arc<RwLock<InsightFaceConfig>>,
    face_indices: Arc<RwLock<HashMap<Uuid, FaceIndex>>>,
    indexed_faces: Arc<RwLock<HashMap<Uuid, Vec<IndexedFace>>>>,
    embedding_cache: Arc<RwLock<HashMap<Uuid, Vec<f32>>>>,
    is_initialized: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone)]
pub enum InsightFaceError {
    NotInitialized,
    InvalidImage(String),
    DetectionFailed(String),
    IndexNotFound(String),
    FaceNotFound(String),
    EmbeddingError(String),
    ConfigError(String),
    IoError(String),
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
