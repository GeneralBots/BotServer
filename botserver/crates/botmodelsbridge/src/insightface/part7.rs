use super::*;

impl InsightFaceService {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(InsightFaceConfig::default())),
            face_indices: Arc::new(RwLock::new(HashMap::new())),
            indexed_faces: Arc::new(RwLock::new(HashMap::new())),
            embedding_cache: Arc::new(RwLock::new(HashMap::new())),
            is_initialized: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_config(config: InsightFaceConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            face_indices: Arc::new(RwLock::new(HashMap::new())),
            indexed_faces: Arc::new(RwLock::new(HashMap::new())),
            embedding_cache: Arc::new(RwLock::new(HashMap::new())),
            is_initialized: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn initialize(&self) -> Result<(), InsightFaceError> {
        let config = self.config.read().await;

        log::info!(
            "Initializing InsightFace with model: {:?}, det_size: {:?}",
            config.model,
            config.det_size
        );

        let mut initialized = self.is_initialized.write().await;
        *initialized = true;

        log::info!("InsightFace initialized successfully");
        Ok(())
    }

    pub async fn is_initialized(&self) -> bool {
        *self.is_initialized.read().await
    }

    pub async fn detect_faces(&self, request: DetectionRequest) -> Result<DetectionResponse, InsightFaceError> {
        if !self.is_initialized().await {
            return Err(InsightFaceError::NotInitialized);
        }

        let start_time = std::time::Instant::now();
        let request_id = Uuid::new_v4();

        let (width, height) = self.get_image_dimensions(&request.image_data, &request.image_format)?;

        let faces = self.run_detection(&request).await?;

        let config = self.config.read().await;

        Ok(DetectionResponse {
            request_id,
            faces,
            image_width: width,
            image_height: height,
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            model_used: config.model.clone(),
        })
    }

    async fn run_detection(&self, request: &DetectionRequest) -> Result<Vec<DetectedFace>, InsightFaceError> {
        let mut faces = Vec::new();
        let config = self.config.read().await;

        let simulated_face_count = 1;

        for i in 0..simulated_face_count {
            let face_id = Uuid::new_v4();

            let bbox = BoundingBox {
                x: 100.0 + (i as f32 * 150.0),
                y: 100.0,
                width: 120.0,
                height: 150.0,
                confidence: 0.95 - (i as f32 * 0.05),
            };

            let landmarks = if request.options.detect_landmarks {
                Some(FaceLandmarks {
                    points_2d: vec![
                        Point2D { x: 130.0, y: 140.0 },
                        Point2D { x: 170.0, y: 140.0 },
                        Point2D { x: 150.0, y: 170.0 },
                        Point2D { x: 135.0, y: 200.0 },
                        Point2D { x: 165.0, y: 200.0 },
                    ],
                    points_3d: None,
                    landmark_type: LandmarkType::FivePoint,
                })
            } else {
                None
            };

            let embedding = if request.options.extract_embedding {
                Some(self.generate_embedding(512))
            } else {
                None
            };

            let (age, gender) = if request.options.detect_age_gender && config.enable_age_gender {
                (Some(28), Some(Gender::Male))
            } else {
                (None, None)
            };

            let pose = if request.options.detect_pose && config.enable_pose {
                Some(FacePose {
                    yaw: 5.0,
                    pitch: -3.0,
                    roll: 2.0,
                })
            } else {
                None
            };

            let quality = if request.options.compute_quality {
                Some(FaceQuality {
                    overall_score: 0.85,
                    brightness: 0.9,
                    sharpness: 0.8,
                    occlusion: 0.05,
                    blur: 0.1,
                })
            } else {
                None
            };

            faces.push(DetectedFace {
                id: face_id,
                bbox,
                detection_score: 0.95,
                landmarks,
                embedding: embedding.clone(),
                age,
                gender,
                pose,
                quality,
                attributes: HashMap::new(),
            });

            if let Some(emb) = embedding {
                let mut cache = self.embedding_cache.write().await;
                cache.insert(face_id, emb);
            }
        }

        Ok(faces)
    }

    pub(super) fn generate_embedding(&self, dimension: usize) -> Vec<f32> {
        use std::f32::consts::PI;
        let mut embedding = Vec::with_capacity(dimension);
        for i in 0..dimension {
            let value = ((i as f32 * PI / 100.0).sin() + 1.0) / 2.0;
            embedding.push(value);
        }
        self.normalize_embedding(&mut embedding);
        embedding
    }

    pub(super) fn normalize_embedding(&self, embedding: &mut [f32]) {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }
    }

    pub(super) fn get_image_dimensions(&self, _data: &[u8], _format: &ImageFormat) -> Result<(u32, u32), InsightFaceError> {
        Ok((640, 480))
    }

    pub async fn verify_faces(
        &self,
        face1_embedding: &[f32],
        face2_embedding: &[f32],
        threshold: Option<f32>,
    ) -> Result<FaceVerificationResult, InsightFaceError> {
        let start_time = std::time::Instant::now();

        let distance = self.compute_distance(face1_embedding, face2_embedding, &DistanceMetric::Cosine);
        let similarity = 1.0 - distance;
        let thresh = threshold.unwrap_or(0.4);

        Ok(FaceVerificationResult {
            face1_id: Uuid::new_v4(),
            face2_id: Uuid::new_v4(),
            is_same_person: similarity >= thresh,
            similarity,
            distance,
            threshold_used: thresh,
            verification_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    pub async fn search_faces(
        &self,
        query_embedding: &[f32],
        index_id: Uuid,
        top_k: u32,
        threshold: Option<f32>,
    ) -> Result<FaceRecognitionResult, InsightFaceError> {
        let start_time = std::time::Instant::now();

        let indexed_faces = self.indexed_faces.read().await;
        let faces = indexed_faces.get(&index_id).ok_or_else(|| {
            InsightFaceError::IndexNotFound(format!("Index {} not found", index_id))
        })?;

        let thresh = threshold.unwrap_or(0.0);
        let mut matches: Vec<FaceMatch> = Vec::new();

        for face in faces {
            let distance = self.compute_distance(query_embedding, &face.embedding, &DistanceMetric::Cosine);
            let similarity = 1.0 - distance;

            if similarity >= thresh {
                matches.push(FaceMatch {
                    face_id: face.face_id,
                    identity_id: face.identity_id.clone(),
                    similarity,
                    distance,
                    metadata: face.metadata.clone(),
                });
            }
        }

        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(top_k as usize);

        Ok(FaceRecognitionResult {
            query_face_id: Uuid::new_v4(),
            matches,
            search_time_ms: start_time.elapsed().as_millis() as u64,
            total_candidates: faces.len() as u32,
        })
    }

    pub async fn create_index(&self, name: &str, description: Option<&str>) -> Result<FaceIndex, InsightFaceError> {
        let index = FaceIndex {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            face_count: 0,
            embedding_dimension: 512,
            distance_metric: DistanceMetric::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut indices = self.face_indices.write().await;
        indices.insert(index.id, index.clone());

        let mut indexed_faces = self.indexed_faces.write().await;
        indexed_faces.insert(index.id, Vec::new());

        Ok(index)
    }

    pub async fn add_face_to_index(
        &self,
        index_id: Uuid,
        embedding: Vec<f32>,
        identity_id: Option<&str>,
        metadata: HashMap<String, String>,
    ) -> Result<IndexedFace, InsightFaceError> {
        let mut indices = self.face_indices.write().await;
        let index = indices.get_mut(&index_id).ok_or_else(|| {
            InsightFaceError::IndexNotFound(format!("Index {} not found", index_id))
        })?;

        let indexed_face = IndexedFace {
            face_id: Uuid::new_v4(),
            index_id,
            identity_id: identity_id.map(|s| s.to_string()),
            embedding,
            metadata,
            created_at: Utc::now(),
        };

        let mut indexed_faces = self.indexed_faces.write().await;
        let faces = indexed_faces.get_mut(&index_id).ok_or_else(|| {
            InsightFaceError::IndexNotFound(format!("Index {} not found", index_id))
        })?;

        faces.push(indexed_face.clone());
        index.face_count = faces.len() as u32;
        index.updated_at = Utc::now();

        Ok(indexed_face)
    }
}
