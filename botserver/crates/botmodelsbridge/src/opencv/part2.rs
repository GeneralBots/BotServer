use super::*;

impl Default for OpenCvFaceDetector {
    fn default() -> Self {
        Self::new(OpenCvDetectorConfig::default())
    }
}
