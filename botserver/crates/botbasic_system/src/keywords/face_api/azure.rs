//! Azure Face API Types
//!
//! This module contains Azure-specific response types and conversions.

use botmodelsbridge::face_api::{BoundingBox, DetectedFace, EmotionScores, FaceAttributes, FaceLandmarks, Gender, GlassesType, Point2D};

// ============================================================================
// Azure API Response Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AzureFaceResponse {
    face_id: Option<String>,
    face_rectangle: AzureFaceRectangle,
    face_landmarks: Option<AzureFaceLandmarks>,
    face_attributes: Option<AzureFaceAttributes>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzureFaceRectangle {
    top: f32,
    left: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzureFaceLandmarks {
    pupil_left: Option<AzurePoint>,
    pupil_right: Option<AzurePoint>,
    nose_tip: Option<AzurePoint>,
    mouth_left: Option<AzurePoint>,
    mouth_right: Option<AzurePoint>,
    eyebrow_left_outer: Option<AzurePoint>,
    eyebrow_left_inner: Option<AzurePoint>,
    eyebrow_right_outer: Option<AzurePoint>,
    eyebrow_right_inner: Option<AzurePoint>,
}

#[derive(Debug, Clone, Deserialize)]
struct AzurePoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzureFaceAttributes {
    age: Option<f32>,
    gender: Option<String>,
    smile: Option<f32>,
    glasses: Option<String>,
    emotion: Option<AzureEmotion>,
}

#[derive(Debug, Clone, Deserialize)]
struct AzureEmotion {
    anger: f32,
    contempt: f32,
    disgust: f32,
    fear: f32,
    happiness: f32,
    neutral: f32,
    sadness: f32,
    surprise: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AzureVerifyResponse {
    pub confidence: f64,
}

impl AzureFaceResponse {
    pub(crate) fn into_detected_face(self) -> DetectedFace {
        let face_id = self.face_id
            .and_then(|id| Uuid::parse_str(&id).ok())
            .unwrap_or_else(Uuid::new_v4);

        let landmarks = self.face_landmarks.map(|lm| {
            FaceLandmarks {
                left_eye: lm.pupil_left.map(|p| Point2D { x: p.x, y: p.y })
                    .unwrap_or(Point2D { x: 0.0, y: 0.0 }),
                right_eye: lm.pupil_right.map(|p| Point2D { x: p.x, y: p.y })
                    .unwrap_or(Point2D { x: 0.0, y: 0.0 }),
                nose_tip: lm.nose_tip.map(|p| Point2D { x: p.x, y: p.y })
                    .unwrap_or(Point2D { x: 0.0, y: 0.0 }),
                mouth_left: lm.mouth_left.map(|p| Point2D { x: p.x, y: p.y })
                    .unwrap_or(Point2D { x: 0.0, y: 0.0 }),
                mouth_right: lm.mouth_right.map(|p| Point2D { x: p.x, y: p.y })
                    .unwrap_or(Point2D { x: 0.0, y: 0.0 }),
                left_eyebrow_left: lm.eyebrow_left_outer.map(|p| Point2D { x: p.x, y: p.y }),
                left_eyebrow_right: lm.eyebrow_left_inner.map(|p| Point2D { x: p.x, y: p.y }),
                right_eyebrow_left: lm.eyebrow_right_inner.map(|p| Point2D { x: p.x, y: p.y }),
                right_eyebrow_right: lm.eyebrow_right_outer.map(|p| Point2D { x: p.x, y: p.y }),
            }
        });

        let attributes = self.face_attributes.map(|attrs| {
            let gender = attrs.gender.as_ref().map(|g| {
                match g.to_lowercase().as_str() {
                    "male" => Gender::Male,
                    "female" => Gender::Female,
                    _ => Gender::Unknown,
                }
            });

            let emotion = attrs.emotion.map(|e| EmotionScores {
                anger: e.anger,
                contempt: e.contempt,
                disgust: e.disgust,
                fear: e.fear,
                happiness: e.happiness,
                neutral: e.neutral,
                sadness: e.sadness,
                surprise: e.surprise,
            });

            let glasses = attrs.glasses.as_ref().map(|g| {
                match g.to_lowercase().as_str() {
                    "noглasses" | "noglasses" => GlassesType::NoGlasses,
                    "readingglasses" => GlassesType::ReadingGlasses,
                    "sunglasses" => GlassesType::Sunglasses,
                    "swimminggoggles" => GlassesType::SwimmingGoggles,
                    _ => GlassesType::NoGlasses,
                }
            });

            FaceAttributes {
                age: attrs.age,
                gender,
                emotion,
                glasses,
                facial_hair: None,
                head_pose: None,
                smile: attrs.smile,
                blur: None,
                exposure: None,
                noise: None,
                occlusion: None,
            }
        });

        DetectedFace {
            id: face_id,
            bounding_box: BoundingBox {
                left: self.face_rectangle.left,
                top: self.face_rectangle.top,
                width: self.face_rectangle.width,
                height: self.face_rectangle.height,
            },
            confidence: 1.0,
            landmarks,
            attributes,
            embedding: None,
        }
    }
}

use serde::Deserialize;
use uuid::Uuid;