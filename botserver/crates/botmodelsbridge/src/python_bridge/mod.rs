pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::io::{BufRead, BufReader, Write};
pub(crate) use std::process::{Child, ChildStdin, ChildStdout, Stdio};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::{Mutex, RwLock};
pub(crate) use botsecurity::command_guard::SafeCommand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonFaceDetection {
    pub face_id: String,
    pub bounding_box: PythonBoundingBox,
    pub confidence: f64,
    pub landmarks: Option<PythonFaceLandmarks>,
    pub attributes: Option<PythonFaceAttributes>,
    pub embedding: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonBoundingBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonFaceLandmarks {
    pub left_eye: PythonPoint,
    pub right_eye: PythonPoint,
    pub nose: PythonPoint,
    pub left_mouth: PythonPoint,
    pub right_mouth: PythonPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonFaceAttributes {
    pub age: Option<f64>,
    pub gender: Option<String>,
    pub emotion: Option<String>,
    pub glasses: Option<bool>,
    pub beard: Option<bool>,
    pub mask: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonVerificationResult {
    pub is_match: bool,
    pub confidence: f64,
    pub distance: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum PythonCommand {
    #[serde(rename = "detect")]
    Detect {
        image_base64: String,
        detect_landmarks: bool,
        detect_attributes: bool,
        model: String,
    },
    #[serde(rename = "verify")]
    Verify {
        image1_base64: String,
        image2_base64: String,
        model: String,
    },
    #[serde(rename = "extract_embedding")]
    ExtractEmbedding {
        image_base64: String,
        model: String,
    },
    #[serde(rename = "analyze")]
    Analyze {
        image_base64: String,
        attributes: Vec<String>,
    },
    #[serde(rename = "health")]
    Health,
    #[serde(rename = "shutdown")]
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum PythonResponse {
    #[serde(rename = "success")]
    Success { data: serde_json::Value },
    #[serde(rename = "error")]
    Error { message: String, code: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum PythonModel {
    #[default]
    MediaPipe,
    DeepFace,
    FaceRecognition,
    InsightFace,
    Dlib,
    OpenCV,
}

#[derive(Debug, Clone)]
pub struct PythonBridgeConfig {
    pub python_path: String,
    pub script_path: String,
    pub model: PythonModel,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub gpu_enabled: bool,
}

struct PythonProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

pub struct PythonFaceBridge {
    config: PythonBridgeConfig,
    process: Arc<Mutex<Option<PythonProcess>>>,
    is_healthy: Arc<RwLock<bool>>,
    embeddings_cache: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PythonBridgeError {
    ProcessSpawnFailed(String),
    ProcessNotRunning,
    CommunicationError(String),
    SerializationError(String),
    DeserializationError(String),
    HealthCheckFailed,
    DetectionFailed(String),
    VerificationFailed(String),
    EmbeddingFailed(String),
    AnalysisFailed(String),
    Timeout,
    ModelNotSupported(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_model_from_str() {
        assert_eq!("mediapipe".parse::<PythonModel>(), Ok(PythonModel::MediaPipe));
        assert_eq!("deepface".parse::<PythonModel>(), Ok(PythonModel::DeepFace));
        assert!("unknown".parse::<PythonModel>().is_err());
    }

    #[test]
    fn test_python_model_as_str() {
        assert_eq!(PythonModel::MediaPipe.as_str(), "mediapipe");
        assert_eq!(PythonModel::DeepFace.as_str(), "deepface");
    }

    #[test]
    fn test_cosine_distance() {
        let bridge = PythonFaceBridge::new(PythonBridgeConfig::default());

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((bridge.cosine_distance(&a, &b) - 0.0).abs() < 0.0001);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!((bridge.cosine_distance(&c, &d) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_error_display() {
        let error = PythonBridgeError::ProcessNotRunning;
        assert!(error.to_string().contains("not running"));

        let error = PythonBridgeError::DetectionFailed("test error".to_string());
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_command_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let cmd = PythonCommand::Health;
        let json = serde_json::to_string(&cmd)?;
        assert!(json.contains("health"));
        Ok(())
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

pub use part6::*;
