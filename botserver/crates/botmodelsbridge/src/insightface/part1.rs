use super::*;

impl Default for InsightFaceConfig {
    fn default() -> Self {
        Self {
            model: InsightFaceModel::default(),
            det_size: (640, 640),
            det_thresh: 0.5,
            ctx_id: 0,
            model_path: None,
            enable_recognition: true,
            enable_age_gender: true,
            enable_landmarks: true,
            enable_pose: true,
            max_faces: None,
            gpu_enabled: false,
        }
    }
}
