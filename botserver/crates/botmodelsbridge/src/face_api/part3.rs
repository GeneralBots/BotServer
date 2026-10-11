use super::*;

impl FaceApiService {
    pub(crate) async fn detect_faces_insightface(
        &self,
        request: FaceDetectionRequest,
    ) -> Result<FaceDetectionResponse, FaceApiError> {
        let start = std::time::Instant::now();

        let face = DetectedFace {
            id: Uuid::new_v4(),
            bounding_box: BoundingBox {
                left: 110.0,
                top: 75.0,
                width: 170.0,
                height: 210.0,
            },
            confidence: 0.9543,
            landmarks: if request.return_landmarks {
                Some(FaceLandmarks {
                    left_eye: Point2D { x: 155.0, y: 135.0 },
                    right_eye: Point2D { x: 230.0, y: 137.0 },
                    nose_tip: Point2D { x: 192.0, y: 182.0 },
                    mouth_left: Point2D { x: 158.0, y: 230.0 },
                    mouth_right: Point2D { x: 226.0, y: 232.0 },
                    left_eyebrow_left: Some(Point2D { x: 135.0, y: 115.0 }),
                    left_eyebrow_right: Some(Point2D { x: 170.0, y: 113.0 }),
                    right_eyebrow_left: Some(Point2D { x: 215.0, y: 114.0 }),
                    right_eyebrow_right: Some(Point2D { x: 250.0, y: 116.0 }),
                })
            } else {
                None
            },
            attributes: if request.return_attributes {
                Some(FaceAttributes {
                    age: Some(28.0),
                    gender: Some(Gender::Female),
                    emotion: Some(EmotionScores {
                        anger: 0.01,
                        contempt: 0.01,
                        disgust: 0.01,
                        fear: 0.01,
                        happiness: 0.8,
                        neutral: 0.1,
                        sadness: 0.02,
                        surprise: 0.04,
                    }),
                    smile: Some(0.72),
                    glasses: Some(GlassesType::NoGlasses),
                    facial_hair: None,
                    head_pose: Some(HeadPose { pitch: 1.0, roll: 0.5, yaw: -2.0 }),
                    blur: Some(BlurLevel::Low),
                    exposure: Some(ExposureLevel::GoodExposure),
                    noise: Some(NoiseLevel::Low),
                    occlusion: None,
                })
            } else {
                None
            },
            embedding: Some(vec![0.08; 512]),
        };

        let mut cache = self.face_cache.write().await;
        cache.insert(face.id, face.clone());

        Ok(FaceDetectionResponse {
            faces: vec![face],
            image_width: 640,
            image_height: 480,
            processing_time_ms: start.elapsed().as_millis() as u64,
            provider: FaceApiProvider::InsightFace,
        })
    }

    pub(super) fn convert_azure_face(&self, azure: AzureFaceResponse) -> DetectedFace {
        DetectedFace {
            id: azure
                .face_id
                .and_then(|s| Uuid::parse_str(&s).ok())
                .unwrap_or_else(Uuid::new_v4),
            bounding_box: BoundingBox {
                left: azure.face_rectangle.left as f32,
                top: azure.face_rectangle.top as f32,
                width: azure.face_rectangle.width as f32,
                height: azure.face_rectangle.height as f32,
            },
            confidence: 1.0,
            landmarks: azure.face_landmarks.map(|fl| FaceLandmarks {
                left_eye: Point2D {
                    x: fl.pupil_left.x,
                    y: fl.pupil_left.y,
                },
                right_eye: Point2D {
                    x: fl.pupil_right.x,
                    y: fl.pupil_right.y,
                },
                nose_tip: Point2D {
                    x: fl.nose_tip.x,
                    y: fl.nose_tip.y,
                },
                mouth_left: Point2D {
                    x: fl.mouth_left.x,
                    y: fl.mouth_left.y,
                },
                mouth_right: Point2D {
                    x: fl.mouth_right.x,
                    y: fl.mouth_right.y,
                },
                left_eyebrow_left: None,
                left_eyebrow_right: None,
                right_eyebrow_left: None,
                right_eyebrow_right: None,
            }),
            attributes: azure.face_attributes.map(|fa| FaceAttributes {
                age: fa.age,
                gender: fa.gender.map(|g| match g.to_lowercase().as_str() {
                    "male" => Gender::Male,
                    "female" => Gender::Female,
                    _ => Gender::Unknown,
                }),
                emotion: fa.emotion.map(|e| EmotionScores {
                    anger: e.anger,
                    contempt: e.contempt,
                    disgust: e.disgust,
                    fear: e.fear,
                    happiness: e.happiness,
                    neutral: e.neutral,
                    sadness: e.sadness,
                    surprise: e.surprise,
                }),
                glasses: fa.glasses.map(|g| match g.to_lowercase().as_str() {
                    "noglasses" => GlassesType::NoGlasses,
                    "readingglasses" => GlassesType::ReadingGlasses,
                    "sunglasses" => GlassesType::Sunglasses,
                    "swimminggoggles" => GlassesType::SwimmingGoggles,
                    _ => GlassesType::NoGlasses,
                }),
                facial_hair: fa.facial_hair.map(|fh| FacialHair {
                    beard: fh.beard,
                    moustache: fh.moustache,
                    sideburns: fh.sideburns,
                }),
                head_pose: fa.head_pose.map(|hp| HeadPose {
                    pitch: hp.pitch,
                    roll: hp.roll,
                    yaw: hp.yaw,
                }),
                smile: fa.smile,
                blur: None,
                exposure: None,
                noise: None,
                occlusion: None,
            }),
            embedding: None,
        }
    }

    pub(super) fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        (dot_product / (norm_a * norm_b)) as f64
    }

    pub fn provider(&self) -> FaceApiProvider {
        self.config.provider
    }
}
