use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceMatch {
    pub similarity: f32,
    pub face: RekognitionFace,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareFacesMatch {
    pub similarity: f32,
    pub face: RekognitionFace,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectFacesRequest {
    pub image_bytes: Vec<u8>,
    pub attributes: Vec<FaceAttribute>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaceAttribute {
    Default,
    All,
    AgeRange,
    Beard,
    Emotions,
    Eyeglasses,
    EyesOpen,
    Gender,
    MouthOpen,
    Mustache,
    Smile,
    Sunglasses,
    FaceOccluded,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectFacesResponse {
    pub faces: Vec<RekognitionFace>,
    pub orientation_correction: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFacesRequest {
    pub collection_id: String,
    pub image_bytes: Vec<u8>,
    pub external_image_id: Option<String>,
    pub max_faces: Option<u32>,
    pub quality_filter: Option<QualityFilter>,
    pub detection_attributes: Vec<FaceAttribute>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFacesResponse {
    pub face_records: Vec<FaceRecord>,
    pub orientation_correction: Option<String>,
    pub face_model_version: String,
    pub unindexed_faces: Vec<UnindexedFace>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceRecord {
    pub face: IndexedFace,
    pub face_detail: RekognitionFace,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnindexedFace {
    pub reasons: Vec<UnindexedFaceReason>,
    pub face_detail: RekognitionFace,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UnindexedFaceReason {
    ExceedMaxFaces,
    ExtremePose,
    LowBrightness,
    LowSharpness,
    LowConfidence,
    SmallBoundingBox,
    LowFaceQuality,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacesRequest {
    pub collection_id: String,
    pub face_id: String,
    pub max_faces: Option<u32>,
    pub face_match_threshold: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacesResponse {
    pub searched_face_id: String,
    pub face_matches: Vec<FaceMatch>,
    pub face_model_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacesByImageRequest {
    pub collection_id: String,
    pub image_bytes: Vec<u8>,
    pub max_faces: Option<u32>,
    pub face_match_threshold: Option<f32>,
    pub quality_filter: Option<QualityFilter>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacesByImageResponse {
    pub searched_face_bounding_box: BoundingBox,
    pub searched_face_confidence: f32,
    pub face_matches: Vec<FaceMatch>,
    pub face_model_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareFacesRequest {
    pub source_image_bytes: Vec<u8>,
    pub target_image_bytes: Vec<u8>,
    pub similarity_threshold: Option<f32>,
    pub quality_filter: Option<QualityFilter>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareFacesResponse {
    pub source_image_face: Option<RekognitionFace>,
    pub face_matches: Vec<CompareFacesMatch>,
    pub unmatched_faces: Vec<RekognitionFace>,
    pub source_image_orientation_correction: Option<String>,
    pub target_image_orientation_correction: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFacesRequest {
    pub collection_id: String,
    pub face_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFacesResponse {
    pub deleted_faces: Vec<String>,
    pub undeleted_faces: Vec<UnsuccessfulFaceDeletion>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsuccessfulFaceDeletion {
    pub face_id: String,
    pub reasons: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessSessionRequest {
    pub settings: Option<LivenessSettings>,
    pub client_request_token: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessSettings {
    pub output_config: Option<LivenessOutputConfig>,
    pub audit_images_limit: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessOutputConfig {
    pub s3_bucket: String,
    pub s3_key_prefix: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessSessionResponse {
    pub session_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessSession {
    pub session_id: String,
    pub status: LivenessSessionStatus,
    pub settings: Option<LivenessSettings>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFaceLivenessSessionResultsResponse {
    pub session_id: String,
    pub status: LivenessSessionStatus,
    pub confidence: Option<f32>,
    pub reference_image: Option<AuditImage>,
    pub audit_images: Vec<AuditImage>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LivenessSessionStatus {
    Created,
    InProgress,
    Succeeded,
    Failed,
    Expired,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditImage {
    pub bytes: Option<Vec<u8>>,
    pub s3_object: Option<S3Object>,
    pub bounding_box: Option<BoundingBox>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Object {
    pub bucket: String,
    pub name: String,
    pub version: Option<String>,
}
pub struct RekognitionService {
    pub(crate) config: Arc<RwLock<RekognitionConfig>>,
    pub(crate) collections: Arc<RwLock<HashMap<String, FaceCollection>>>,
    pub(crate) indexed_faces: Arc<RwLock<HashMap<String, Vec<IndexedFace>>>>,
    pub(crate) face_details: Arc<RwLock<HashMap<String, RekognitionFace>>>,
}
