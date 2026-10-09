use super::*;

impl std::fmt::Display for OpenCvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "OpenCV face detector not initialized"),
            Self::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
            Self::ModelNotFound(path) => write!(f, "Model file not found: {path}"),
            Self::InvalidImage(msg) => write!(f, "Invalid image: {msg}"),
            Self::NoFaceDetected(source) => write!(f, "No face detected in {source}"),
            Self::ProcessingError(msg) => write!(f, "Processing error: {msg}"),
            Self::UnsupportedFormat(fmt) => write!(f, "Unsupported image format: {fmt}"),
        }
    }
}
impl std::error::Error for OpenCvError {}
pub async fn create_opencv_detector() -> Result<OpenCvFaceDetector, OpenCvError> {
    let config = OpenCvDetectorConfig {
        use_dnn: true,
        confidence_threshold: 0.7,
        min_face_size: (50, 50),
        ..Default::default()
    };

    let mut detector = OpenCvFaceDetector::new(config);
    detector.initialize()?;
    Ok(detector)
}
pub async fn detect_faces_opencv(
    image_data: &[u8],
    detect_landmarks: bool,
    detect_attributes: bool,
) -> Result<Vec<OpenCvFaceDetection>, OpenCvError> {
    let detector = create_opencv_detector().await?;
    detector
        .detect_faces(image_data, detect_landmarks, detect_attributes)
        .await
}
pub async fn verify_faces_opencv(
    image1_data: &[u8],
    image2_data: &[u8],
) -> Result<FaceVerificationResult, OpenCvError> {
    let detector = create_opencv_detector().await?;
    detector.verify_faces(image1_data, image2_data).await
}
