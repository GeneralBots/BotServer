pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::Path;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCvFaceDetection {
    pub face_id: String,
    pub bounding_box: BoundingBox,
    pub confidence: f64,
    pub landmarks: Option<FaceLandmarks>,
    pub attributes: Option<FaceAttributes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceLandmarks {
    pub left_eye: Point,
    pub right_eye: Point,
    pub nose: Point,
    pub left_mouth_corner: Point,
    pub right_mouth_corner: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceAttributes {
    pub estimated_age: Option<f64>,
    pub gender: Option<String>,
    pub emotion: Option<String>,
    pub glasses: Option<bool>,
    pub face_quality: Option<f64>,
    pub head_pose: Option<HeadPose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadPose {
    pub pitch: f64,
    pub roll: f64,
    pub yaw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceVerificationResult {
    pub is_match: bool,
    pub confidence: f64,
    pub distance: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceEmbedding {
    pub face_id: String,
    pub embedding: Vec<f64>,
    pub model_version: String,
}

#[derive(Debug, Clone)]
pub struct OpenCvDetectorConfig {
    pub cascade_path: Option<String>,
    pub dnn_model_path: Option<String>,
    pub dnn_config_path: Option<String>,
    pub scale_factor: f64,
    pub min_neighbors: i32,
    pub min_face_size: (i32, i32),
    pub max_face_size: Option<(i32, i32)>,
    pub use_dnn: bool,
    pub confidence_threshold: f64,
    pub nms_threshold: f64,
}

pub struct OpenCvFaceDetector {
    config: OpenCvDetectorConfig,
    initialized: bool,
    face_embeddings_cache: HashMap<String, FaceEmbedding>,
}

#[derive(Debug)]
struct ImageInfo {
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpenCvError {
    NotInitialized,
    ConfigError(String),
    ModelNotFound(String),
    InvalidImage(String),
    NoFaceDetected(String),
    ProcessingError(String),
    UnsupportedFormat(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detector_initialization() {
        let config = OpenCvDetectorConfig::default();
        let detector = OpenCvFaceDetector::new(config);
        assert!(!detector.is_initialized());
    }

    #[test]
    fn test_embedding_distance() {
        let detector = OpenCvFaceDetector::default();

        let embedding1 = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let embedding2 = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        let distance = detector.compute_embedding_distance(&embedding1, &embedding2);
        assert!(distance < 0.0001);

        let embedding3 = vec![0.5, 0.4, 0.3, 0.2, 0.1];
        let distance2 = detector.compute_embedding_distance(&embedding1, &embedding3);
        assert!(distance2 > 0.0);
    }

    #[test]
    fn test_bounding_box_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let bbox = BoundingBox {
            x: 100,
            y: 150,
            width: 200,
            height: 250,
        };

        let json = serde_json::to_string(&bbox)?;
        let deserialized: BoundingBox = serde_json::from_str(&json)?;

        assert_eq!(deserialized.x, bbox.x);
        assert_eq!(deserialized.y, bbox.y);
        assert_eq!(deserialized.width, bbox.width);
        assert_eq!(deserialized.height, bbox.height);
        Ok(())
    }

    #[test]
    fn test_error_display() {
        let error = OpenCvError::NotInitialized;
        assert!(error.to_string().contains("not initialized"));

        let error = OpenCvError::ModelNotFound("/path/to/model".to_string());
        assert!(error.to_string().contains("/path/to/model"));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part5::*;
