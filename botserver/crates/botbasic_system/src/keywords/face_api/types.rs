//! Face API Types
//!
//! This module contains all type definitions for the Face API keywords including
//! image sources, face sources, detection options, and attribute types.

use botmodelsbridge::face_api::DetectedFace;
use serde::{Deserialize, Serialize};

// ============================================================================
// Keyword Definitions
// ============================================================================

/// DETECT FACES keyword - Detect faces in an image
///
/// Syntax:
///   faces = DETECT FACES image_url
///   faces = DETECT FACES image_url WITH OPTIONS options
///
/// Examples:
///   faces = DETECT FACES "https://example.com/photo.jpg"
///   faces = DETECT FACES photo WITH OPTIONS { "return_landmarks": true, "return_attributes": true }
///
/// Returns: Array of detected faces with bounding boxes and optional attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectFacesKeyword {
    pub image_source: ImageSource,
    pub options: DetectionOptions,
}

/// VERIFY FACE keyword - Verify if two faces belong to the same person
///
/// Syntax:
///   result = VERIFY FACE face1 AGAINST face2
///   result = VERIFY FACE image1 AGAINST image2
///
/// Examples:
///   match = VERIFY FACE saved_face AGAINST new_photo
///   result = VERIFY FACE "https://example.com/id.jpg" AGAINST camera_capture
///
/// Returns: Verification result with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyFaceKeyword {
    pub face1: FaceSource,
    pub face2: FaceSource,
    pub options: VerificationOptions,
}

/// ANALYZE FACE keyword - Analyze face attributes in detail
///
/// Syntax:
///   analysis = ANALYZE FACE image_url
///   analysis = ANALYZE FACE face_id WITH ATTRIBUTES attributes_list
///
/// Examples:
///   analysis = ANALYZE FACE photo WITH ATTRIBUTES ["age", "emotion", "gender"]
///   result = ANALYZE FACE captured_image
///
/// Returns: Detailed face analysis including emotions, age, gender, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeFaceKeyword {
    pub source: FaceSource,
    pub attributes: Vec<FaceAttributeType>,
    pub options: AnalysisOptions,
}

/// FIND SIMILAR FACES keyword - Find similar faces in a collection
///
/// Syntax:
///   similar = FIND SIMILAR FACES TO face IN collection
///
/// Examples:
///   matches = FIND SIMILAR FACES TO suspect_photo IN employee_database
///
/// Returns: Array of similar faces with similarity scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarFacesKeyword {
    pub target_face: FaceSource,
    pub collection_name: String,
    pub max_results: usize,
    pub min_confidence: f32,
}

/// GROUP FACES keyword - Group faces by similarity
///
/// Syntax:
///   groups = GROUP FACES face_list
///
/// Examples:
///   groups = GROUP FACES detected_faces
///
/// Returns: Groups of similar faces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupFacesKeyword {
    pub faces: Vec<FaceSource>,
    pub options: GroupingOptions,
}

// ============================================================================
// Supporting Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageSource {
    Url(String),
    Base64(String),
    FilePath(String),
    Variable(String),
    Binary(Vec<u8>),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FaceSource {
    Image(ImageSource),
    FaceId(Uuid),
    DetectedFace(Box<DetectedFace>),
    Embedding(Vec<f32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionOptions {
    #[serde(default = "default_true")]
    pub return_face_id: bool,
    #[serde(default)]
    pub return_landmarks: Option<bool>,
    #[serde(default)]
    pub return_attributes: Option<bool>,
    #[serde(default)]
    pub return_embedding: bool,
    #[serde(default)]
    pub detection_model: Option<String>,
    #[serde(default)]
    pub recognition_model: Option<String>,
    #[serde(default)]
    pub max_faces: Option<usize>,
    #[serde(default = "default_min_face_size")]
    pub min_face_size: u32,
}

fn default_true() -> bool {
    true
}

fn _default_max_faces() -> usize {
    100
}

fn default_min_face_size() -> u32 {
    36
}

impl Default for DetectionOptions {
    fn default() -> Self {
        Self {
            return_face_id: true,
            return_landmarks: Some(false),
            return_attributes: Some(false),
            return_embedding: false,
            detection_model: None,
            recognition_model: None,
            max_faces: Some(100),
            min_face_size: 36,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationOptions {
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,
    #[serde(default)]
    pub recognition_model: Option<String>,
    #[serde(default)]
    pub threshold: Option<f64>,
}

fn default_confidence_threshold() -> f64 {
    0.6
}

impl Default for VerificationOptions {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.8,
            recognition_model: None,
            threshold: Some(0.8),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    #[serde(default = "default_true")]
    pub return_landmarks: bool,
    #[serde(default)]
    pub detection_model: Option<String>,
    #[serde(default)]
    pub recognition_model: Option<String>,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            return_landmarks: true,
            detection_model: None,
            recognition_model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupingOptions {
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
}

fn default_similarity_threshold() -> f32 {
    0.5
}

impl Default for GroupingOptions {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FaceAttributeType {
    Age,
    Gender,
    Emotion,
    Smile,
    Glasses,
    FacialHair,
    HeadPose,
    Blur,
    Exposure,
    Noise,
    Occlusion,
    Accessories,
    Hair,
    Makeup,
    QualityForRecognition,
}

use uuid::Uuid;