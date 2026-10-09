//! Face API Service
//!
//! This module contains the main FaceApiService implementation with support for
//! multiple providers: Azure Face API, AWS Rekognition, OpenCV, and InsightFace.

pub(crate) use super::azure::AzureFaceResponse;
pub(crate) use super::error::FaceApiError;
pub(crate) use super::results::{FaceAnalysisResult, FaceDetectionResult, FaceVerificationResult};
pub(crate) use super::types::{AnalysisOptions, DetectionOptions, FaceAttributeType, FaceSource, ImageSource, VerificationOptions};
pub(crate) use botmodelsbridge::{BoundingBox, DetectedFace, EmotionScores, FaceApiConfig, FaceApiProvider, FaceAttributes, FaceLandmarks, Gender, GlassesType, Point2D};

pub struct FaceApiService {
    config: FaceApiConfig,
    client: reqwest::Client,
    face_cache: Arc<RwLock<HashMap<Uuid, DetectedFace>>>,
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;

use part1::*;
