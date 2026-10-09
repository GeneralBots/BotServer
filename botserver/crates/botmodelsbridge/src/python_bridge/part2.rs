use super::*;

impl Default for PythonBridgeConfig {
    fn default() -> Self {
        Self {
            python_path: "python3".to_string(),
            script_path: "scripts/face_detection.py".to_string(),
            model: PythonModel::MediaPipe,
            timeout_seconds: 30,
            max_retries: 3,
            gpu_enabled: false,
        }
    }
}
