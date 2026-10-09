use super::*;

impl InsightFaceService {
    pub async fn remove_face_from_index(&self, index_id: Uuid, face_id: Uuid) -> Result<(), InsightFaceError> {
        let mut indices = self.face_indices.write().await;
        let index = indices.get_mut(&index_id).ok_or_else(|| {
            InsightFaceError::IndexNotFound(format!("Index {} not found", index_id))
        })?;

        let mut indexed_faces = self.indexed_faces.write().await;
        let faces = indexed_faces.get_mut(&index_id).ok_or_else(|| {
            InsightFaceError::IndexNotFound(format!("Index {} not found", index_id))
        })?;

        let original_len = faces.len();
        faces.retain(|f| f.face_id != face_id);

        if faces.len() == original_len {
            return Err(InsightFaceError::FaceNotFound(format!("Face {} not found in index", face_id)));
        }

        index.face_count = faces.len() as u32;
        index.updated_at = Utc::now();

        Ok(())
    }

    pub async fn delete_index(&self, index_id: Uuid) -> Result<(), InsightFaceError> {
        let mut indices = self.face_indices.write().await;
        if indices.remove(&index_id).is_none() {
            return Err(InsightFaceError::IndexNotFound(format!("Index {} not found", index_id)));
        }

        let mut indexed_faces = self.indexed_faces.write().await;
        indexed_faces.remove(&index_id);

        Ok(())
    }

    pub async fn get_index(&self, index_id: Uuid) -> Option<FaceIndex> {
        let indices = self.face_indices.read().await;
        indices.get(&index_id).cloned()
    }

    pub async fn list_indices(&self) -> Vec<FaceIndex> {
        let indices = self.face_indices.read().await;
        indices.values().cloned().collect()
    }

    pub async fn cluster_faces(
        &self,
        embeddings: &[(Uuid, Vec<f32>)],
        config: ClusteringConfig,
    ) -> Result<Vec<FaceCluster>, InsightFaceError> {
        if embeddings.is_empty() {
            return Ok(Vec::new());
        }

        let mut clusters: Vec<FaceCluster> = Vec::new();
        let mut assigned: Vec<bool> = vec![false; embeddings.len()];

        for (i, (face_id, embedding)) in embeddings.iter().enumerate() {
            if assigned[i] {
                continue;
            }

            let mut cluster_faces = vec![*face_id];
            let mut cluster_embeddings = vec![embedding.clone()];
            assigned[i] = true;

            for (j, (other_face_id, other_embedding)) in embeddings.iter().enumerate().skip(i + 1) {
                if assigned[j] {
                    continue;
                }

                let distance = self.compute_distance(embedding, other_embedding, &DistanceMetric::Cosine);
                if distance < config.distance_threshold {
                    cluster_faces.push(*other_face_id);
                    cluster_embeddings.push(other_embedding.clone());
                    assigned[j] = true;
                }
            }

            if cluster_faces.len() >= config.min_cluster_size as usize {
                let centroid = self.compute_centroid(&cluster_embeddings);

                clusters.push(FaceCluster {
                    cluster_id: Uuid::new_v4(),
                    identity_id: None,
                    face_ids: cluster_faces,
                    centroid,
                    quality_score: 0.8,
                    created_at: Utc::now(),
                });
            }
        }

        Ok(clusters)
    }

    pub(super) fn compute_distance(&self, emb1: &[f32], emb2: &[f32], metric: &DistanceMetric) -> f32 {
        match metric {
            DistanceMetric::Cosine => {
                let dot: f32 = emb1.iter().zip(emb2.iter()).map(|(a, b)| a * b).sum();
                let norm1: f32 = emb1.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm2: f32 = emb2.iter().map(|x| x * x).sum::<f32>().sqrt();

                if norm1 > 0.0 && norm2 > 0.0 {
                    1.0 - (dot / (norm1 * norm2))
                } else {
                    1.0
                }
            }
            DistanceMetric::Euclidean => {
                emb1.iter()
                    .zip(emb2.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f32>()
                    .sqrt()
            }
            DistanceMetric::DotProduct => {
                -emb1.iter().zip(emb2.iter()).map(|(a, b)| a * b).sum::<f32>()
            }
        }
    }

    pub(super) fn compute_centroid(&self, embeddings: &[Vec<f32>]) -> Vec<f32> {
        if embeddings.is_empty() {
            return Vec::new();
        }

        let dim = embeddings[0].len();
        let mut centroid = vec![0.0f32; dim];

        for emb in embeddings {
            for (i, val) in emb.iter().enumerate() {
                centroid[i] += val;
            }
        }

        let n = embeddings.len() as f32;
        for val in centroid.iter_mut() {
            *val /= n;
        }

        self.normalize_embedding(&mut centroid);
        centroid
    }

    pub async fn get_embedding_from_cache(&self, face_id: Uuid) -> Option<Vec<f32>> {
        let cache = self.embedding_cache.read().await;
        cache.get(&face_id).cloned()
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.embedding_cache.write().await;
        cache.clear();
    }

    pub async fn update_config(&self, config: InsightFaceConfig) {
        let mut current = self.config.write().await;
        *current = config;
    }

    pub async fn get_config(&self) -> InsightFaceConfig {
        self.config.read().await.clone()
    }
}
