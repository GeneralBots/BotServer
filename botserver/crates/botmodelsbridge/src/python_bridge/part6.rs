use super::*;

impl std::fmt::Display for PythonBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProcessSpawnFailed(msg) => write!(f, "Failed to spawn Python process: {msg}"),
            Self::ProcessNotRunning => write!(f, "Python process is not running"),
            Self::CommunicationError(msg) => write!(f, "Communication error: {msg}"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            Self::DeserializationError(msg) => write!(f, "Deserialization error: {msg}"),
            Self::HealthCheckFailed => write!(f, "Python bridge health check failed"),
            Self::DetectionFailed(msg) => write!(f, "Face detection failed: {msg}"),
            Self::VerificationFailed(msg) => write!(f, "Face verification failed: {msg}"),
            Self::EmbeddingFailed(msg) => write!(f, "Embedding extraction failed: {msg}"),
            Self::AnalysisFailed(msg) => write!(f, "Face analysis failed: {msg}"),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::ModelNotSupported(model) => write!(f, "Model not supported: {model}"),
        }
    }
}
impl std::error::Error for PythonBridgeError {}
pub async fn create_python_bridge(model: Option<PythonModel>) -> Result<PythonFaceBridge, PythonBridgeError> {
    let config = PythonBridgeConfig {
        model: model.unwrap_or_default(),
        ..Default::default()
    };

    let bridge = PythonFaceBridge::new(config);
    bridge.start().await?;
    Ok(bridge)
}
pub async fn detect_faces_python(
    image_data: &[u8],
    detect_landmarks: bool,
    detect_attributes: bool,
) -> Result<Vec<PythonFaceDetection>, PythonBridgeError> {
    let bridge = create_python_bridge(None).await?;
    let result = bridge.detect_faces(image_data, detect_landmarks, detect_attributes).await;
    let _ = bridge.stop().await;
    result
}
pub async fn verify_faces_python(
    image1_data: &[u8],
    image2_data: &[u8],
) -> Result<PythonVerificationResult, PythonBridgeError> {
    let bridge = create_python_bridge(None).await?;
    let result = bridge.verify_faces(image1_data, image2_data).await;
    let _ = bridge.stop().await;
    result
}
