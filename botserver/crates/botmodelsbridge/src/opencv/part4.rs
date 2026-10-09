use super::*;

impl OpenCvFaceDetector {
    pub(super) fn compute_embedding_distance(&self, embedding1: &[f64], embedding2: &[f64]) -> f64 {
        if embedding1.len() != embedding2.len() {
            return 1.0;
        }

        let distance: f64 = embedding1
            .iter()
            .zip(embedding2.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum();

        distance.sqrt()
    }

    pub fn cache_embedding(&mut self, person_id: &str, embedding: FaceEmbedding) {
        self.face_embeddings_cache.insert(person_id.to_string(), embedding);
    }

    pub fn get_cached_embedding(&self, person_id: &str) -> Option<&FaceEmbedding> {
        self.face_embeddings_cache.get(person_id)
    }

    pub fn clear_embedding_cache(&mut self) {
        self.face_embeddings_cache.clear();
    }

    pub async fn find_matching_faces(
        &self,
        image_data: &[u8],
        threshold: f64,
    ) -> Result<Vec<(String, f64)>, OpenCvError> {
        if !self.initialized {
            return Err(OpenCvError::NotInitialized);
        }

        let faces = self.detect_faces(image_data, false, false).await?;

        if faces.is_empty() {
            return Ok(Vec::new());
        }

        let query_embedding = self.extract_embedding(image_data, &faces[0].bounding_box).await?;

        let mut matches = Vec::new();

        for (person_id, cached_embedding) in &self.face_embeddings_cache {
            let distance = self.compute_embedding_distance(
                &query_embedding.embedding,
                &cached_embedding.embedding,
            );

            if distance < threshold {
                let confidence = 1.0 - (distance / threshold);
                matches.push((person_id.clone(), confidence));
            }
        }

        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(matches)
    }

    pub(super) fn decode_image_info(&self, image_data: &[u8]) -> Result<ImageInfo, OpenCvError> {
        if image_data.len() < 24 {
            return Err(OpenCvError::InvalidImage("Image data too small".to_string()));
        }

        if image_data.starts_with(&[0x89, 0x50, 0x4E, 0x47])
            && image_data.len() >= 24 {
                let width = u32::from_be_bytes([
                    image_data[16],
                    image_data[17],
                    image_data[18],
                    image_data[19],
                ]) as i32;
                let height = u32::from_be_bytes([
                    image_data[20],
                    image_data[21],
                    image_data[22],
                    image_data[23],
                ]) as i32;
                return Ok(ImageInfo {
                    width,
                    height,
                });
            }

        if image_data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Ok(ImageInfo {
                width: 640,
                height: 480,
            });
        }

        if image_data.starts_with(b"BM")
            && image_data.len() >= 26 {
                let width = i32::from_le_bytes([
                    image_data[18],
                    image_data[19],
                    image_data[20],
                    image_data[21],
                ]);
                let height = i32::from_le_bytes([
                    image_data[22],
                    image_data[23],
                    image_data[24],
                    image_data[25],
                ])
                .abs();
                return Ok(ImageInfo {
                    width,
                    height,
                });
            }

        Ok(ImageInfo {
            width: 640,
            height: 480,
        })
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn get_config(&self) -> &OpenCvDetectorConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: OpenCvDetectorConfig) -> Result<(), OpenCvError> {
        self.config = config;
        self.initialized = false;
        self.initialize()
    }
}
