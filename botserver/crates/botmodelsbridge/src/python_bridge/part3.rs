use super::*;

impl PythonFaceBridge {
    pub fn new(config: PythonBridgeConfig) -> Self {
        Self {
            config,
            process: Arc::new(Mutex::new(None)),
            is_healthy: Arc::new(RwLock::new(false)),
            embeddings_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self) -> Result<(), PythonBridgeError> {
        let mut process_guard = self.process.lock().await;

        if process_guard.is_some() {
            return Ok(());
        }

        let python_cmd = std::path::Path::new(&self.config.python_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("python3");

        let mut command = SafeCommand::new(python_cmd).map_err(|e| PythonBridgeError::ProcessSpawnFailed(e.to_string()))?;
        command = command.arg(&self.config.script_path).map_err(|e| PythonBridgeError::ProcessSpawnFailed(e.to_string()))?;
        command = command.arg("--model").map_err(|e| PythonBridgeError::ProcessSpawnFailed(e.to_string()))?;
        command = command.arg(self.config.model.as_str()).map_err(|e| PythonBridgeError::ProcessSpawnFailed(e.to_string()))?;
        command = command.arg(if self.config.gpu_enabled { "--gpu" } else { "--cpu" }).map_err(|e| PythonBridgeError::ProcessSpawnFailed(e.to_string()))?;
        command = command.stdout(Stdio::piped());
        command = command.stderr(Stdio::piped());

        let mut child = command.spawn()
            .map_err(|e| PythonBridgeError::ProcessSpawnFailed(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PythonBridgeError::ProcessSpawnFailed("Failed to capture stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PythonBridgeError::ProcessSpawnFailed("Failed to capture stdout".to_string()))?;

        *process_guard = Some(PythonProcess {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        });

        drop(process_guard);

        let health_result = self.health_check().await;
        let mut is_healthy = self.is_healthy.write().await;
        *is_healthy = health_result.is_ok();

        if health_result.is_err() {
            return Err(PythonBridgeError::HealthCheckFailed);
        }

        log::info!("Python face detection bridge started successfully");
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), PythonBridgeError> {
        let mut process_guard = self.process.lock().await;

        if let Some(mut process) = process_guard.take() {
            let shutdown_cmd = PythonCommand::Shutdown;
            let cmd_json = serde_json::to_string(&shutdown_cmd)
                .map_err(|e| PythonBridgeError::SerializationError(e.to_string()))?;

            let _ = writeln!(process.stdin, "{cmd_json}");
            let _ = process.stdin.flush();

            std::thread::sleep(std::time::Duration::from_millis(500));

            let _ = process.child.kill();
            let _ = process.child.wait();
        }

        let mut is_healthy = self.is_healthy.write().await;
        *is_healthy = false;

        log::info!("Python face detection bridge stopped");
        Ok(())
    }

    pub async fn health_check(&self) -> Result<bool, PythonBridgeError> {
        let response = self.send_command(PythonCommand::Health).await?;

        match response {
            PythonResponse::Success { .. } => Ok(true),
            PythonResponse::Error { message, .. } => {
                log::warn!("Python bridge health check failed: {message}");
                Err(PythonBridgeError::HealthCheckFailed)
            }
        }
    }

    pub async fn detect_faces(
        &self,
        image_data: &[u8],
        detect_landmarks: bool,
        detect_attributes: bool,
    ) -> Result<Vec<PythonFaceDetection>, PythonBridgeError> {
        let image_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            image_data,
        );

        let command = PythonCommand::Detect {
            image_base64,
            detect_landmarks,
            detect_attributes,
            model: self.config.model.as_str().to_string(),
        };

        let response = self.send_command(command).await?;

        match response {
            PythonResponse::Success { data } => {
                let faces: Vec<PythonFaceDetection> = serde_json::from_value(data)
                    .map_err(|e| PythonBridgeError::DeserializationError(e.to_string()))?;
                Ok(faces)
            }
            PythonResponse::Error { message, .. } => {
                Err(PythonBridgeError::DetectionFailed(message))
            }
        }
    }

    pub async fn verify_faces(
        &self,
        image1_data: &[u8],
        image2_data: &[u8],
    ) -> Result<PythonVerificationResult, PythonBridgeError> {
        let image1_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            image1_data,
        );
        let image2_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            image2_data,
        );

        let command = PythonCommand::Verify {
            image1_base64,
            image2_base64,
            model: self.config.model.as_str().to_string(),
        };

        let response = self.send_command(command).await?;

        match response {
            PythonResponse::Success { data } => {
                let result: PythonVerificationResult = serde_json::from_value(data)
                    .map_err(|e| PythonBridgeError::DeserializationError(e.to_string()))?;
                Ok(result)
            }
            PythonResponse::Error { message, .. } => {
                Err(PythonBridgeError::VerificationFailed(message))
            }
        }
    }

    pub async fn extract_embedding(
        &self,
        image_data: &[u8],
    ) -> Result<Vec<f64>, PythonBridgeError> {
        let image_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            image_data,
        );

        let command = PythonCommand::ExtractEmbedding {
            image_base64,
            model: self.config.model.as_str().to_string(),
        };

        let response = self.send_command(command).await?;

        match response {
            PythonResponse::Success { data } => {
                let embedding: Vec<f64> = serde_json::from_value(data)
                    .map_err(|e| PythonBridgeError::DeserializationError(e.to_string()))?;
                Ok(embedding)
            }
            PythonResponse::Error { message, .. } => {
                Err(PythonBridgeError::EmbeddingFailed(message))
            }
        }
    }

    pub async fn analyze_face(
        &self,
        image_data: &[u8],
        attributes: Vec<String>,
    ) -> Result<PythonFaceAttributes, PythonBridgeError> {
        let image_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            image_data,
        );

        let command = PythonCommand::Analyze {
            image_base64,
            attributes,
        };

        let response = self.send_command(command).await?;

        match response {
            PythonResponse::Success { data } => {
                let attrs: PythonFaceAttributes = serde_json::from_value(data)
                    .map_err(|e| PythonBridgeError::DeserializationError(e.to_string()))?;
                Ok(attrs)
            }
            PythonResponse::Error { message, .. } => {
                Err(PythonBridgeError::AnalysisFailed(message))
            }
        }
    }

    pub async fn cache_embedding(&self, person_id: &str, embedding: Vec<f64>) {
        let mut cache = self.embeddings_cache.write().await;
        cache.insert(person_id.to_string(), embedding);
    }

    pub async fn get_cached_embedding(&self, person_id: &str) -> Option<Vec<f64>> {
        let cache = self.embeddings_cache.read().await;
        cache.get(person_id).cloned()
    }

    pub async fn find_matching_face(
        &self,
        image_data: &[u8],
        threshold: f64,
    ) -> Result<Option<(String, f64)>, PythonBridgeError> {
        let query_embedding = self.extract_embedding(image_data).await?;
        let cache = self.embeddings_cache.read().await;

        let mut best_match: Option<(String, f64)> = None;

        for (person_id, cached_embedding) in cache.iter() {
            let distance = self.cosine_distance(&query_embedding, cached_embedding);
            let similarity = 1.0 - distance;

            if similarity >= threshold {
                if let Some((_, best_similarity)) = &best_match {
                    if similarity > *best_similarity {
                        best_match = Some((person_id.clone(), similarity));
                    }
                } else {
                    best_match = Some((person_id.clone(), similarity));
                }
            }
        }

        Ok(best_match)
    }

    pub(super) fn cosine_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 1.0;
        }

        let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0;
        }

        1.0 - (dot_product / (norm_a * norm_b))
    }

    async fn send_command(&self, command: PythonCommand) -> Result<PythonResponse, PythonBridgeError> {
        let mut process_guard = self.process.lock().await;

        let process = process_guard
            .as_mut()
            .ok_or(PythonBridgeError::ProcessNotRunning)?;

        let cmd_json = serde_json::to_string(&command)
            .map_err(|e| PythonBridgeError::SerializationError(e.to_string()))?;

        writeln!(process.stdin, "{cmd_json}")
            .map_err(|e| PythonBridgeError::CommunicationError(e.to_string()))?;

        process.stdin.flush()
            .map_err(|e| PythonBridgeError::CommunicationError(e.to_string()))?;

        let mut response_line = String::new();
        process.stdout.read_line(&mut response_line)
            .map_err(|e| PythonBridgeError::CommunicationError(e.to_string()))?;

        let response: PythonResponse = serde_json::from_str(&response_line)
            .map_err(|e| PythonBridgeError::DeserializationError(e.to_string()))?;

        Ok(response)
    }
}
