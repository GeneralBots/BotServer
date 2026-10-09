use super::*;

impl RekognitionService {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(RekognitionConfig::default())),
            collections: Arc::new(RwLock::new(HashMap::new())),
            indexed_faces: Arc::new(RwLock::new(HashMap::new())),
            face_details: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_config(config: RekognitionConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            collections: Arc::new(RwLock::new(HashMap::new())),
            indexed_faces: Arc::new(RwLock::new(HashMap::new())),
            face_details: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn detect_faces(&self, request: DetectFacesRequest) -> Result<DetectFacesResponse, RekognitionError> {
        if request.image_bytes.is_empty() {
            return Err(RekognitionError::InvalidImage("Image bytes cannot be empty".to_string()));
        }

        let config = self.config.read().await;
        let include_all = request.attributes.contains(&FaceAttribute::All);

        let mut faces = Vec::new();
        let simulated_face_count = 1;

        for _ in 0..simulated_face_count {
            let face = self.create_simulated_face(&config, include_all);
            faces.push(face);
        }

        Ok(DetectFacesResponse {
            faces,
            orientation_correction: Some("ROTATE_0".to_string()),
        })
    }

    pub(super) fn create_simulated_face(&self, config: &RekognitionConfig, include_all: bool) -> RekognitionFace {
        let landmarks = vec![
            Landmark { landmark_type: LandmarkType::EyeLeft, x: 0.35, y: 0.35 },
            Landmark { landmark_type: LandmarkType::EyeRight, x: 0.65, y: 0.35 },
            Landmark { landmark_type: LandmarkType::Nose, x: 0.5, y: 0.55 },
            Landmark { landmark_type: LandmarkType::MouthLeft, x: 0.38, y: 0.75 },
            Landmark { landmark_type: LandmarkType::MouthRight, x: 0.62, y: 0.75 },
        ];

        let age_range = if include_all || config.enable_age_range {
            Some(AgeRange { low: 25, high: 35 })
        } else {
            None
        };

        let smile = if include_all || config.enable_smile_detection {
            Some(Smile { value: true, confidence: 95.5 })
        } else {
            None
        };

        let eyeglasses = if include_all || config.enable_eyeglasses_detection {
            Some(Eyeglasses { value: false, confidence: 98.2 })
        } else {
            None
        };

        let sunglasses = if include_all || config.enable_sunglasses_detection {
            Some(Sunglasses { value: false, confidence: 99.1 })
        } else {
            None
        };

        let gender = if include_all || config.enable_gender_detection {
            Some(Gender {
                value: GenderValue::Male,
                confidence: 97.8,
            })
        } else {
            None
        };

        let beard = if include_all || config.enable_beard_detection {
            Some(Beard { value: false, confidence: 96.3 })
        } else {
            None
        };

        let mustache = if include_all || config.enable_mustache_detection {
            Some(Mustache { value: false, confidence: 97.1 })
        } else {
            None
        };

        let eyes_open = if include_all || config.enable_eyes_open_detection {
            Some(EyesOpen { value: true, confidence: 99.5 })
        } else {
            None
        };

        let mouth_open = if include_all || config.enable_mouth_open_detection {
            Some(MouthOpen { value: false, confidence: 98.7 })
        } else {
            None
        };

        let emotions = if include_all || config.enable_emotions {
            vec![
                Emotion { emotion_type: EmotionType::Happy, confidence: 85.2 },
                Emotion { emotion_type: EmotionType::Calm, confidence: 12.5 },
                Emotion { emotion_type: EmotionType::Surprised, confidence: 2.3 },
            ]
        } else {
            Vec::new()
        };

        let face_occluded = if include_all || config.enable_face_occluded_detection {
            Some(FaceOccluded { value: false, confidence: 98.9 })
        } else {
            None
        };

        RekognitionFace {
            face_id: Some(Uuid::new_v4().to_string()),
            bounding_box: BoundingBox {
                width: 0.25,
                height: 0.35,
                left: 0.35,
                top: 0.15,
            },
            confidence: 99.8,
            image_id: Some(Uuid::new_v4().to_string()),
            external_image_id: None,
            landmarks,
            pose: Pose {
                roll: 2.5,
                yaw: -5.0,
                pitch: 3.2,
            },
            quality: FaceQuality {
                brightness: 85.5,
                sharpness: 92.3,
            },
            age_range,
            smile,
            eyeglasses,
            sunglasses,
            gender,
            beard,
            mustache,
            eyes_open,
            mouth_open,
            emotions,
            face_occluded,
        }
    }

    pub async fn create_collection(&self, collection_id: &str) -> Result<FaceCollection, RekognitionError> {
        let mut collections = self.collections.write().await;

        if collections.contains_key(collection_id) {
            return Err(RekognitionError::CollectionAlreadyExists(collection_id.to_string()));
        }

        let collection = FaceCollection {
            collection_id: collection_id.to_string(),
            collection_arn: Some(format!("arn:aws:rekognition:us-east-1:123456789012:collection/{collection_id}")),
            face_count: 0,
            face_model_version: "6.0".to_string(),
            creation_timestamp: Some(Utc::now()),
        };

        collections.insert(collection_id.to_string(), collection.clone());

        let mut indexed_faces = self.indexed_faces.write().await;
        indexed_faces.insert(collection_id.to_string(), Vec::new());

        Ok(collection)
    }

    pub async fn delete_collection(&self, collection_id: &str) -> Result<(), RekognitionError> {
        let mut collections = self.collections.write().await;

        if collections.remove(collection_id).is_none() {
            return Err(RekognitionError::CollectionNotFound(collection_id.to_string()));
        }

        let mut indexed_faces = self.indexed_faces.write().await;
        indexed_faces.remove(collection_id);

        Ok(())
    }

    pub async fn list_collections(&self) -> Vec<FaceCollection> {
        let collections = self.collections.read().await;
        collections.values().cloned().collect()
    }

    pub async fn describe_collection(&self, collection_id: &str) -> Result<FaceCollection, RekognitionError> {
        let collections = self.collections.read().await;

        collections
            .get(collection_id)
            .cloned()
            .ok_or_else(|| RekognitionError::CollectionNotFound(collection_id.to_string()))
    }

    pub async fn index_faces(&self, request: IndexFacesRequest) -> Result<IndexFacesResponse, RekognitionError> {
        if request.image_bytes.is_empty() {
            return Err(RekognitionError::InvalidImage("Image bytes cannot be empty".to_string()));
        }

        let mut collections = self.collections.write().await;
        let collection = collections
            .get_mut(&request.collection_id)
            .ok_or_else(|| RekognitionError::CollectionNotFound(request.collection_id.clone()))?;

        let config = self.config.read().await;
        let max_faces = request.max_faces.unwrap_or(config.max_faces);

        let mut face_records = Vec::new();
        let simulated_face_count = 1.min(max_faces as usize);

        for _ in 0..simulated_face_count {
            let face_id = Uuid::new_v4().to_string();
            let image_id = Uuid::new_v4().to_string();

            let indexed_face = IndexedFace {
                face_id: face_id.clone(),
                bounding_box: BoundingBox {
                    width: 0.25,
                    height: 0.35,
                    left: 0.35,
                    top: 0.15,
                },
                image_id: image_id.clone(),
                external_image_id: request.external_image_id.clone(),
                confidence: 99.8,
            };

            let face_detail = self.create_simulated_face(&config, true);

            face_records.push(FaceRecord {
                face: indexed_face.clone(),
                face_detail: face_detail.clone(),
            });

            let mut indexed_faces = self.indexed_faces.write().await;
            if let Some(faces) = indexed_faces.get_mut(&request.collection_id) {
                faces.push(indexed_face);
            }

            let mut face_details = self.face_details.write().await;
            face_details.insert(face_id, face_detail);
        }

        collection.face_count += face_records.len() as u64;

        Ok(IndexFacesResponse {
            face_records,
            orientation_correction: Some("ROTATE_0".to_string()),
            face_model_version: "6.0".to_string(),
            unindexed_faces: Vec::new(),
        })
    }

    pub async fn search_faces(&self, request: SearchFacesRequest) -> Result<SearchFacesResponse, RekognitionError> {
        let indexed_faces = self.indexed_faces.read().await;
        let collection_faces = indexed_faces
            .get(&request.collection_id)
            .ok_or_else(|| RekognitionError::CollectionNotFound(request.collection_id.clone()))?;

        if !collection_faces.iter().any(|f| f.face_id == request.face_id) {
            return Err(RekognitionError::FaceNotFound(request.face_id.clone()));
        }

        let config = self.config.read().await;
        let threshold = request.face_match_threshold.unwrap_or(config.face_match_threshold);
        let max_faces = request.max_faces.unwrap_or(10);

        let mut face_matches = Vec::new();

        for face in collection_faces.iter().filter(|f| f.face_id != request.face_id) {
            let similarity = 85.0 + (rand_float() * 15.0);

            if similarity >= threshold {
                let face_details = self.face_details.read().await;
                if let Some(detail) = face_details.get(&face.face_id) {
                    face_matches.push(FaceMatch {
                        similarity,
                        face: detail.clone(),
                    });
                }
            }
        }

        face_matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        face_matches.truncate(max_faces as usize);

        Ok(SearchFacesResponse {
            searched_face_id: request.face_id,
            face_matches,
            face_model_version: "6.0".to_string(),
        })
    }

    pub async fn search_faces_by_image(&self, request: SearchFacesByImageRequest) -> Result<SearchFacesByImageResponse, RekognitionError> {
        if request.image_bytes.is_empty() {
            return Err(RekognitionError::InvalidImage("Image bytes cannot be empty".to_string()));
        }

        let indexed_faces = self.indexed_faces.read().await;
        let collection_faces = indexed_faces
            .get(&request.collection_id)
            .ok_or_else(|| RekognitionError::CollectionNotFound(request.collection_id.clone()))?;

        let config = self.config.read().await;
        let threshold = request.face_match_threshold.unwrap_or(config.face_match_threshold);
        let max_faces = request.max_faces.unwrap_or(10);

        let searched_face_bounding_box = BoundingBox {
            width: 0.25,
            height: 0.35,
            left: 0.35,
            top: 0.15,
        };

        let mut face_matches = Vec::new();

        for face in collection_faces.iter() {
            let similarity = 80.0 + (rand_float() * 20.0);

            if similarity >= threshold {
                let face_details = self.face_details.read().await;
                if let Some(detail) = face_details.get(&face.face_id) {
                    face_matches.push(FaceMatch {
                        similarity,
                        face: detail.clone(),
                    });
                }
            }
        }

        face_matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        face_matches.truncate(max_faces as usize);

        Ok(SearchFacesByImageResponse {
            searched_face_bounding_box,
            searched_face_confidence: 99.5,
            face_matches,
            face_model_version: "6.0".to_string(),
        })
    }
}
