//! Face API Result Types
//!
//! This module contains all result types returned by Face API operations.

use botmodelsbridge::face_api::{DetectedFace, FaceAttributes};
use serde::{Deserialize, Serialize};

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetectionResult {
    pub success: bool,
    pub faces: Vec<DetectedFace>,
    pub face_count: usize,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub processing_time_ms: u64,
    pub error: Option<String>,
}

impl FaceDetectionResult {
    pub fn success(faces: Vec<DetectedFace>, processing_time_ms: u64) -> Self {
        let face_count = faces.len();
        Self {
            success: true,
            faces,
            face_count,
            image_width: None,
            image_height: None,
            processing_time_ms,
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            faces: Vec::new(),
            face_count: 0,
            image_width: None,
            image_height: None,
            processing_time_ms: 0,
            error: Some(message),
        }
    }

    pub fn with_image_size(mut self, width: u32, height: u32) -> Self {
        self.image_width = Some(width);
        self.image_height = Some(height);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceVerificationResult {
    pub success: bool,
    pub is_match: bool,
    pub confidence: f64,
    pub threshold: f64,
    pub face1_id: Option<Uuid>,
    pub face2_id: Option<Uuid>,
    pub processing_time_ms: u64,
    pub error: Option<String>,
}

impl FaceVerificationResult {
    pub fn match_found(confidence: f64, threshold: f64, processing_time_ms: u64) -> Self {
        Self {
            success: true,
            is_match: confidence >= threshold,
            confidence,
            threshold,
            face1_id: None,
            face2_id: None,
            processing_time_ms,
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            is_match: false,
            confidence: 0.0,
            threshold: 0.0,
            face1_id: None,
            face2_id: None,
            processing_time_ms: 0,
            error: Some(message),
        }
    }

    pub fn with_face_ids(mut self, face1_id: Uuid, face2_id: Uuid) -> Self {
        self.face1_id = Some(face1_id);
        self.face2_id = Some(face2_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceAnalysisResult {
    pub success: bool,
    pub face: Option<DetectedFace>,
    pub attributes: Option<FaceAttributes>,
    pub dominant_emotion: Option<String>,
    pub estimated_age: Option<f32>,
    pub gender: Option<String>,
    pub smile_intensity: Option<f32>,
    pub quality_score: Option<f32>,
    pub processing_time_ms: u64,
    pub error: Option<String>,
}

impl FaceAnalysisResult {
    pub fn success(face: DetectedFace, processing_time_ms: u64) -> Self {
        let attributes = face.attributes.clone();
        let dominant_emotion = attributes.as_ref()
            .and_then(|a| a.emotion.as_ref())
            .map(|e| e.dominant_emotion().to_string());
        let estimated_age = attributes.as_ref().and_then(|a| a.age);
        let gender = attributes.as_ref()
            .and_then(|a| a.gender)
            .map(|g| format!("{:?}", g).to_lowercase());
        let smile_intensity = attributes.as_ref().and_then(|a| a.smile);

        Self {
            success: true,
            face: Some(face),
            attributes,
            dominant_emotion,
            estimated_age,
            gender,
            smile_intensity,
            quality_score: None,
            processing_time_ms,
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            face: None,
            attributes: None,
            dominant_emotion: None,
            estimated_age: None,
            gender: None,
            smile_intensity: None,
            quality_score: None,
            processing_time_ms: 0,
            error: Some(message),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarFaceResult {
    pub face_id: Uuid,
    pub confidence: f64,
    pub person_id: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceGroup {
    pub group_id: Uuid,
    pub face_ids: Vec<Uuid>,
    pub representative_face_id: Option<Uuid>,
    pub confidence: f64,
}

use std::collections::HashMap;
use uuid::Uuid;