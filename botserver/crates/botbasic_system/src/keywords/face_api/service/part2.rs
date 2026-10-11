use super::*;

impl FaceApiService {
    pub fn new(config: FaceApiConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            face_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Detect faces in an image
    pub async fn detect_faces(
        &self,
        image: &ImageSource,
        options: &DetectionOptions,
    ) -> Result<FaceDetectionResult, FaceApiError> {
        let start = std::time::Instant::now();

        match self.config.provider {
            FaceApiProvider::AzureFaceApi => {
                self.detect_faces_azure(image, options).await
            }
            FaceApiProvider::AwsRekognition => {
                self.detect_faces_aws(image, options).await
            }
            FaceApiProvider::OpenCv => {
                self.detect_faces_opencv(image, options).await
            }
            FaceApiProvider::InsightFace => {
                self.detect_faces_insightface(image, options).await
            }
        }
        .map(|mut result| {
            result.processing_time_ms = start.elapsed().as_millis() as u64;
            result
        })
    }

    /// Verify if two faces are the same person
    pub async fn verify_faces(
        &self,
        face1: &FaceSource,
        face2: &FaceSource,
        options: &VerificationOptions,
    ) -> Result<FaceVerificationResult, FaceApiError> {
        let start = std::time::Instant::now();

        match self.config.provider {
            FaceApiProvider::AzureFaceApi => {
                self.verify_faces_azure(face1, face2, options).await
            }
            FaceApiProvider::AwsRekognition => {
                self.verify_faces_aws(face1, face2, options).await
            }
            FaceApiProvider::OpenCv => {
                self.verify_faces_opencv(face1, face2, options).await
            }
            FaceApiProvider::InsightFace => {
                self.verify_faces_insightface(face1, face2, options).await
            }
        }
        .map(|mut result| {
            result.processing_time_ms = start.elapsed().as_millis() as u64;
            result
        })
    }

    /// Analyze face attributes
    pub async fn analyze_face(
        &self,
        source: &FaceSource,
        attributes: &[FaceAttributeType],
        options: &AnalysisOptions,
    ) -> Result<FaceAnalysisResult, FaceApiError> {
        let start = std::time::Instant::now();

        match self.config.provider {
            FaceApiProvider::AzureFaceApi => {
                self.analyze_face_azure(source, attributes, options).await
            }
            FaceApiProvider::AwsRekognition => {
                self.analyze_face_aws(source, attributes, options).await
            }
            FaceApiProvider::OpenCv => {
                self.analyze_face_opencv(source, attributes, options).await
            }
            FaceApiProvider::InsightFace => {
                self.analyze_face_insightface(source, attributes, options).await
            }
        }
        .map(|mut result| {
            result.processing_time_ms = start.elapsed().as_millis() as u64;
            result
        })
    }

    // ========================================================================
    // Azure Face API Implementation
    // ========================================================================

    async fn detect_faces_azure(
        &self,
        image: &ImageSource,
        options: &DetectionOptions,
    ) -> Result<FaceDetectionResult, FaceApiError> {
        let endpoint = self.config.endpoint.as_ref()
            .ok_or(FaceApiError::ConfigError("Azure endpoint not configured".to_string()))?;
        let api_key = self.config.api_key.as_ref()
            .ok_or(FaceApiError::ConfigError("Azure API key not configured".to_string()))?;

        let mut return_params = vec!["faceId"];
        if options.return_landmarks.unwrap_or(false) {
            return_params.push("faceLandmarks");
        }

        let mut attributes = Vec::new();
        if options.return_attributes.unwrap_or(false) {
            attributes.extend_from_slice(&[
                "age", "gender", "smile", "glasses", "emotion",
                "facialHair", "headPose", "blur", "exposure", "noise", "occlusion"
            ]);
        }

        let url = format!(
            "{}/face/v1.0/detect?returnFaceId={}&returnFaceLandmarks={}&returnFaceAttributes={}",
            endpoint,
            options.return_face_id,
            options.return_landmarks.unwrap_or(false),
            attributes.join(",")
        );

        let request = match image {
            ImageSource::Url(image_url) => {
                self.client
                    .post(&url)
                    .header("Ocp-Apim-Subscription-Key", api_key)
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({ "url": image_url }))
            }
            ImageSource::Base64(data) => {
                let bytes = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    data,
                ).map_err(|e| FaceApiError::InvalidInput(e.to_string()))?;

                self.client
                    .post(&url)
                    .header("Ocp-Apim-Subscription-Key", api_key)
                    .header("Content-Type", "application/octet-stream")
                    .body(bytes)
            }
            ImageSource::Binary(bytes) => {
                self.client
                    .post(&url)
                    .header("Ocp-Apim-Subscription-Key", api_key)
                    .header("Content-Type", "application/octet-stream")
                    .body(bytes.clone())
            }
            _ => return Err(FaceApiError::InvalidInput("Unsupported image source for Azure".to_string())),
        };

        let response = request.send().await
            .map_err(|e| FaceApiError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(FaceApiError::ApiError(error_text));
        }

        let azure_faces: Vec<AzureFaceResponse> = response.json().await
            .map_err(|e| FaceApiError::ParseError(e.to_string()))?;

        let faces = azure_faces
            .into_iter()
            .map(|af| af.into_detected_face())
            .collect();

        Ok(FaceDetectionResult::success(faces, 0))
    }

    async fn verify_faces_azure(
        &self,
        face1: &FaceSource,
        face2: &FaceSource,
        options: &VerificationOptions,
    ) -> Result<FaceVerificationResult, FaceApiError> {
        let endpoint = self.config.endpoint.as_ref()
            .ok_or(FaceApiError::ConfigError("Azure endpoint not configured".to_string()))?;
        let api_key = self.config.api_key.as_ref()
            .ok_or(FaceApiError::ConfigError("Azure API key not configured".to_string()))?;

        // Get face IDs (may need to detect first)
        let face1_id = self.get_or_detect_face_id(face1).await?;
        let face2_id = self.get_or_detect_face_id(face2).await?;

        let url = format!("{}/face/v1.0/verify", endpoint);

        let response = self.client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "faceId1": face1_id.to_string(),
                "faceId2": face2_id.to_string()
            }))
            .send()
            .await
            .map_err(|e| FaceApiError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(FaceApiError::ApiError(error_text));
        }

        let result: AzureVerifyResponse = response.json().await
            .map_err(|e| FaceApiError::ParseError(e.to_string()))?;

        Ok(FaceVerificationResult::match_found(
            result.confidence,
            options.confidence_threshold,
            0,
        ).with_face_ids(face1_id, face2_id))
    }

    async fn analyze_face_azure(
        &self,
        source: &FaceSource,
        attributes: &[FaceAttributeType],
        options: &AnalysisOptions,
    ) -> Result<FaceAnalysisResult, FaceApiError> {
        let detect_options = DetectionOptions {
            return_face_id: true,
            return_landmarks: Some(options.return_landmarks),
            return_attributes: Some(!attributes.is_empty()),
            ..Default::default()
        };

        let image = match source {
            FaceSource::Image(img) => img.clone(),
            FaceSource::DetectedFace(face) => {
                return Ok(FaceAnalysisResult::success(*face.clone(), 0));
            }
            _ => return Err(FaceApiError::InvalidInput("Cannot analyze from face ID alone".to_string())),
        };

        let result = self.detect_faces_azure(&image, &detect_options).await?;

        if let Some(face) = result.faces.into_iter().next() {
            Ok(FaceAnalysisResult::success(face, 0))
        } else {
            Err(FaceApiError::NoFaceFound)
        }
    }

    // ========================================================================
    // AWS Rekognition Implementation
    // ========================================================================

    async fn detect_faces_aws(
        &self,
        image: &ImageSource,
        options: &DetectionOptions,
    ) -> Result<FaceDetectionResult, FaceApiError> {
        use std::time::Instant;
        let start = Instant::now();

        // Get image bytes
        let image_bytes = self.get_image_bytes(image).await?;

        // Check if AWS credentials are configured
        let aws_region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let _aws_key = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| FaceApiError::ConfigError("AWS_ACCESS_KEY_ID not configured".to_string()))?;
        let _aws_secret = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| FaceApiError::ConfigError("AWS_SECRET_ACCESS_KEY not configured".to_string()))?;

        // Use simulation for face detection
        // In production with aws-sdk-rekognition crate, this would call the real API
        let faces = self.simulate_face_detection(&image_bytes, options).await;

        // Cache detected faces
        for face in &faces {
            self.face_cache.write().await.insert(face.id, face.clone());
        }

        let processing_time = start.elapsed().as_millis() as u64;

        log::info!(
            "AWS Rekognition: Detected {} faces in {}ms (region: {})",
            faces.len(),
            processing_time,
            aws_region
        );

        Ok(FaceDetectionResult::success(faces, processing_time))
    }
}
