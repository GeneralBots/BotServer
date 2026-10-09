pub(crate) use chrono::{DateTime, Utc};
pub(crate) use reqwest::Client;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FaceApiProvider {
    AzureFaceApi,
    AwsRekognition,
    OpenCv,
    InsightFace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceApiConfig {
    pub provider: FaceApiProvider,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub region: Option<String>,
    pub model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedFace {
    pub id: Uuid,
    pub bounding_box: BoundingBox,
    pub confidence: f64,
    pub landmarks: Option<FaceLandmarks>,
    pub attributes: Option<FaceAttributes>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceLandmarks {
    pub left_eye: Point2D,
    pub right_eye: Point2D,
    pub nose_tip: Point2D,
    pub mouth_left: Point2D,
    pub mouth_right: Point2D,
    pub left_eyebrow_left: Option<Point2D>,
    pub left_eyebrow_right: Option<Point2D>,
    pub right_eyebrow_left: Option<Point2D>,
    pub right_eyebrow_right: Option<Point2D>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceAttributes {
    pub age: Option<f32>,
    pub gender: Option<Gender>,
    pub emotion: Option<EmotionScores>,
    pub glasses: Option<GlassesType>,
    pub facial_hair: Option<FacialHair>,
    pub head_pose: Option<HeadPose>,
    pub smile: Option<f32>,
    pub blur: Option<BlurLevel>,
    pub exposure: Option<ExposureLevel>,
    pub noise: Option<NoiseLevel>,
    pub occlusion: Option<Occlusion>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionScores {
    pub anger: f32,
    pub contempt: f32,
    pub disgust: f32,
    pub fear: f32,
    pub happiness: f32,
    pub neutral: f32,
    pub sadness: f32,
    pub surprise: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GlassesType {
    NoGlasses,
    ReadingGlasses,
    Sunglasses,
    SwimmingGoggles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacialHair {
    pub beard: f32,
    pub moustache: f32,
    pub sideburns: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HeadPose {
    pub pitch: f32,
    pub roll: f32,
    pub yaw: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlurLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExposureLevel {
    UnderExposure,
    GoodExposure,
    OverExposure,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoiseLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Occlusion {
    pub forehead_occluded: bool,
    pub eye_occluded: bool,
    pub mouth_occluded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceVerificationResult {
    pub is_identical: bool,
    pub confidence: f64,
    pub face1_id: Uuid,
    pub face2_id: Uuid,
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceSearchResult {
    pub query_face_id: Uuid,
    pub matches: Vec<FaceMatch>,
    pub searched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceMatch {
    pub face_id: Uuid,
    pub person_id: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetectionRequest {
    pub image_data: Vec<u8>,
    pub image_url: Option<String>,
    pub return_landmarks: bool,
    pub return_attributes: bool,
    pub detection_model: Option<String>,
    pub recognition_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetectionResponse {
    pub faces: Vec<DetectedFace>,
    pub image_width: u32,
    pub image_height: u32,
    pub processing_time_ms: u64,
    pub provider: FaceApiProvider,
}

pub struct FaceApiService {
    config: FaceApiConfig,
    http_client: Client,
    face_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<Uuid, DetectedFace>>>,
}

#[derive(Debug, Clone, Deserialize)]
struct AzureFaceResponse {
    #[serde(rename = "faceId")]
    face_id: Option<String>,
    #[serde(rename = "faceRectangle")]
    face_rectangle: AzureFaceRectangle,
    #[serde(rename = "faceLandmarks")]
    face_landmarks: Option<AzureFaceLandmarks>,
    #[serde(rename = "faceAttributes")]
    face_attributes: Option<AzureFaceAttributes>,
}

#[derive(Debug, Clone, Deserialize)]
struct AzureFaceRectangle {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct AzureFaceLandmarks {
    #[serde(rename = "pupilLeft")]
    pupil_left: AzurePoint,
    #[serde(rename = "pupilRight")]
    pupil_right: AzurePoint,
    #[serde(rename = "noseTip")]
    nose_tip: AzurePoint,
    #[serde(rename = "mouthLeft")]
    mouth_left: AzurePoint,
    #[serde(rename = "mouthRight")]
    mouth_right: AzurePoint,
}

#[derive(Debug, Clone, Deserialize)]
struct AzurePoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct AzureFaceAttributes {
    age: Option<f32>,
    gender: Option<String>,
    smile: Option<f32>,
    #[serde(rename = "facialHair")]
    facial_hair: Option<AzureFacialHair>,
    glasses: Option<String>,
    emotion: Option<AzureEmotion>,
    #[serde(rename = "headPose")]
    head_pose: Option<AzureHeadPose>,
}

#[derive(Debug, Clone, Deserialize)]
struct AzureFacialHair {
    beard: f32,
    moustache: f32,
    sideburns: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct AzureEmotion {
    anger: f32,
    contempt: f32,
    disgust: f32,
    fear: f32,
    happiness: f32,
    neutral: f32,
    sadness: f32,
    surprise: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct AzureHeadPose {
    pitch: f32,
    roll: f32,
    yaw: f32,
}

#[derive(Debug, Clone)]
pub enum FaceApiError {
    ConfigurationError(String),
    NetworkError(String),
    ApiError(String),
    ParseError(String),
    FaceNotFound(Uuid),
    MissingEmbedding(String),
    AttributesNotAvailable,
    ProviderNotImplemented(String),
}

mod part1;
mod part2;
mod part3;
mod part4;

pub use part4::*;
