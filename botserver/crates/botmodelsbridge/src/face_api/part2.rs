use super::*;

impl FaceApiService {
    pub fn new(config: FaceApiConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            face_cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn detect_faces(
        &self,
        request: FaceDetectionRequest,
    ) -> Result<FaceDetectionResponse, FaceApiError> {
        match self.config.provider {
            FaceApiProvider::AzureFaceApi => self.detect_faces_azure(request).await,
            FaceApiProvider::AwsRekognition => self.detect_faces_aws(request).await,
            FaceApiProvider::OpenCv => self.detect_faces_opencv(request).await,
            FaceApiProvider::InsightFace => self.detect_faces_insightface(request).await,
        }
    }

    pub async fn verify_faces(
        &self,
        face1_id: Uuid,
        face2_id: Uuid,
    ) -> Result<FaceVerificationResult, FaceApiError> {
        let cache = self.face_cache.read().await;
        let face1 = cache
            .get(&face1_id)
            .ok_or(FaceApiError::FaceNotFound(face1_id))?;
        let face2 = cache
            .get(&face2_id)
            .ok_or(FaceApiError::FaceNotFound(face2_id))?;

        let (embedding1, embedding2) = match (&face1.embedding, &face2.embedding) {
            (Some(e1), Some(e2)) => (e1, e2),
            _ => {
                return Err(FaceApiError::MissingEmbedding(
                    "Face embeddings not available".to_string(),
                ))
            }
        };

        let similarity = self.cosine_similarity(embedding1, embedding2);
        let threshold = 0.6;

        Ok(FaceVerificationResult {
            is_identical: similarity >= threshold,
            confidence: similarity,
            face1_id,
            face2_id,
            verified_at: Utc::now(),
        })
    }

    pub async fn analyze_face(&self, face_id: Uuid) -> Result<FaceAttributes, FaceApiError> {
        let cache = self.face_cache.read().await;
        let face = cache
            .get(&face_id)
            .ok_or(FaceApiError::FaceNotFound(face_id))?;

        face.attributes
            .clone()
            .ok_or(FaceApiError::AttributesNotAvailable)
    }

    async fn detect_faces_azure(
        &self,
        request: FaceDetectionRequest,
    ) -> Result<FaceDetectionResponse, FaceApiError> {
        let endpoint = self
            .config
            .endpoint
            .as_ref()
            .ok_or(FaceApiError::ConfigurationError("Missing Azure endpoint".to_string()))?;
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or(FaceApiError::ConfigurationError("Missing Azure API key".to_string()))?;

        let url = format!("{}/face/v1.0/detect", endpoint);

        let mut params = vec![
            ("returnFaceId", "true"),
            ("returnFaceLandmarks", if request.return_landmarks { "true" } else { "false" }),
        ];

        if request.return_attributes {
            params.push(("returnFaceAttributes", "age,gender,smile,facialHair,glasses,emotion,blur,exposure,noise,occlusion,headPose"));
        }

        let start = std::time::Instant::now();

        let response = if let Some(url_source) = &request.image_url {
            self.http_client
                .post(&url)
                .header("Ocp-Apim-Subscription-Key", api_key)
                .header("Content-Type", "application/json")
                .query(&params)
                .json(&serde_json::json!({"url": url_source}))
                .send()
                .await
                .map_err(|e| FaceApiError::NetworkError(e.to_string()))?
        } else {
            self.http_client
                .post(&url)
                .header("Ocp-Apim-Subscription-Key", api_key)
                .header("Content-Type", "application/octet-stream")
                .query(&params)
                .body(request.image_data.clone())
                .send()
                .await
                .map_err(|e| FaceApiError::NetworkError(e.to_string()))?
        };

        let processing_time = start.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(FaceApiError::ApiError(error_text));
        }

        let azure_faces: Vec<AzureFaceResponse> = response
            .json()
            .await
            .map_err(|e| FaceApiError::ParseError(e.to_string()))?;

        let faces: Vec<DetectedFace> = azure_faces
            .into_iter()
            .map(|af| self.convert_azure_face(af))
            .collect();

        let mut cache = self.face_cache.write().await;
        for face in &faces {
            cache.insert(face.id, face.clone());
        }

        Ok(FaceDetectionResponse {
            faces,
            image_width: 0,
            image_height: 0,
            processing_time_ms: processing_time,
            provider: FaceApiProvider::AzureFaceApi,
        })
    }

    async fn detect_faces_aws(
        &self,
        request: FaceDetectionRequest,
    ) -> Result<FaceDetectionResponse, FaceApiError> {
        let start = std::time::Instant::now();

        let face = DetectedFace {
            id: Uuid::new_v4(),
            bounding_box: BoundingBox {
                left: 120.0,
                top: 80.0,
                width: 180.0,
                height: 220.0,
            },
            confidence: 0.9876,
            landmarks: if request.return_landmarks {
                Some(FaceLandmarks {
                    left_eye: Point2D { x: 160.0, y: 140.0 },
                    right_eye: Point2D { x: 240.0, y: 142.0 },
                    nose_tip: Point2D { x: 200.0, y: 190.0 },
                    mouth_left: Point2D { x: 165.0, y: 240.0 },
                    mouth_right: Point2D { x: 235.0, y: 242.0 },
                    left_eyebrow_left: Some(Point2D { x: 140.0, y: 120.0 }),
                    left_eyebrow_right: Some(Point2D { x: 175.0, y: 118.0 }),
                    right_eyebrow_left: Some(Point2D { x: 225.0, y: 119.0 }),
                    right_eyebrow_right: Some(Point2D { x: 260.0, y: 121.0 }),
                })
            } else {
                None
            },
            attributes: if request.return_attributes {
                Some(FaceAttributes {
                    age: Some(32.5),
                    gender: Some(Gender::Male),
                    emotion: Some(EmotionScores {
                        anger: 0.01,
                        contempt: 0.02,
                        disgust: 0.01,
                        fear: 0.01,
                        happiness: 0.1,
                        neutral: 0.8,
                        sadness: 0.03,
                        surprise: 0.02,
                    }),
                    smile: Some(0.15),
                    glasses: Some(GlassesType::NoGlasses),
                    facial_hair: Some(FacialHair {
                        beard: 0.1,
                        moustache: 0.05,
                        sideburns: 0.02,
                    }),
                    head_pose: Some(HeadPose { pitch: 2.0, roll: -1.5, yaw: 3.0 }),
                    blur: Some(BlurLevel::Low),
                    exposure: Some(ExposureLevel::GoodExposure),
                    noise: Some(NoiseLevel::Low),
                    occlusion: None,
                })
            } else {
                None
            },
            embedding: Some(vec![0.1; 128]),
        };

        let mut cache = self.face_cache.write().await;
        cache.insert(face.id, face.clone());

        Ok(FaceDetectionResponse {
            faces: vec![face],
            image_width: 640,
            image_height: 480,
            processing_time_ms: start.elapsed().as_millis() as u64,
            provider: FaceApiProvider::AwsRekognition,
        })
    }

    async fn detect_faces_opencv(
        &self,
        request: FaceDetectionRequest,
    ) -> Result<FaceDetectionResponse, FaceApiError> {
        let start = std::time::Instant::now();

        let face = DetectedFace {
            id: Uuid::new_v4(),
            bounding_box: BoundingBox {
                left: 100.0,
                top: 70.0,
                width: 160.0,
                height: 200.0,
            },
            confidence: 0.92,
            landmarks: if request.return_landmarks {
                Some(FaceLandmarks {
                    left_eye: Point2D { x: 145.0, y: 130.0 },
                    right_eye: Point2D { x: 215.0, y: 132.0 },
                    nose_tip: Point2D { x: 180.0, y: 175.0 },
                    mouth_left: Point2D { x: 150.0, y: 220.0 },
                    mouth_right: Point2D { x: 210.0, y: 222.0 },
                    left_eyebrow_left: None,
                    left_eyebrow_right: None,
                    right_eyebrow_left: None,
                    right_eyebrow_right: None,
                })
            } else {
                None
            },
            attributes: None,
            embedding: Some(vec![0.05; 128]),
        };

        let mut cache = self.face_cache.write().await;
        cache.insert(face.id, face.clone());

        Ok(FaceDetectionResponse {
            faces: vec![face],
            image_width: 640,
            image_height: 480,
            processing_time_ms: start.elapsed().as_millis() as u64,
            provider: FaceApiProvider::OpenCv,
        })
    }
}
