use super::*;

impl Default for OpenCvDetectorConfig {
    fn default() -> Self {
        Self {
            cascade_path: None,
            dnn_model_path: None,
            dnn_config_path: None,
            scale_factor: 1.1,
            min_neighbors: 5,
            min_face_size: (30, 30),
            max_face_size: None,
            use_dnn: true,
            confidence_threshold: 0.5,
            nms_threshold: 0.3,
        }
    }
}
