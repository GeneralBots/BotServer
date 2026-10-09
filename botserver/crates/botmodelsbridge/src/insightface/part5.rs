use super::*;

impl Default for DetectionOptions {
    fn default() -> Self {
        Self {
            detect_landmarks: true,
            extract_embedding: true,
            detect_age_gender: true,
            detect_pose: true,
            compute_quality: false,
            min_face_size: Some(20),
            max_faces: None,
            detection_threshold: Some(0.5),
        }
    }
}
