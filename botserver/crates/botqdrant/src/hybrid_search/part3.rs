use super::*;

impl HybridSearchEngine {
    pub fn new(config: HybridSearchConfig, qdrant_url: &str, collection_name: &str) -> Self {
        let mut bm25_index = BM25Index::new();
        bm25_index.set_enabled(config.bm25_enabled);

        info!(
            "Created HybridSearchEngine with fallback BM25 (enabled={})",
            config.bm25_enabled
        );

        Self {
            bm25_index,
            documents: HashMap::new(),
            config,
            qdrant_url: qdrant_url.to_string(),
            collection_name: collection_name.to_string(),
        }
    }

    pub async fn index_document(
        &mut self,
        doc_id: &str,
        content: &str,
        source: &str,
        metadata: HashMap<String, String>,
        embedding: Option<Vec<f32>>,
    ) -> Result<(), String> {
        self.bm25_index.add_document(doc_id, content, source);

        self.documents.insert(
            doc_id.to_string(),
            DocumentEntry {
                content: content.to_string(),
                source: source.to_string(),
                metadata,
            },
        );

        if let Some(emb) = embedding {
            self.upsert_to_qdrant(doc_id, &emb).await?;
        }

        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), String> {
        Ok(())
    }

    pub async fn remove_document(&mut self, doc_id: &str) -> Result<(), String> {
        self.bm25_index.remove_document(doc_id);
        self.documents.remove(doc_id);
        self.delete_from_qdrant(doc_id).await?;
        Ok(())
    }

    pub async fn search(
        &self,
        query: &str,
        query_embedding: Option<Vec<f32>>,
    ) -> Result<Vec<SearchResult>, String> {
        let fetch_count = self.config.max_results * 3;

        let sparse_results: Vec<(String, f32)> = if self.config.use_sparse_search() {
            self.bm25_index
                .search(query, fetch_count)
                .into_iter()
                .map(|(doc_id, _source, score)| (doc_id, score))
                .collect()
        } else {
            Vec::new()
        };

        let dense_results = if self.config.use_dense_search() {
            if let Some(embedding) = query_embedding {
                self.search_qdrant(&embedding, fetch_count).await?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let (results, method) = if sparse_results.is_empty() && dense_results.is_empty() {
            (Vec::new(), SearchMethod::Hybrid)
        } else if sparse_results.is_empty() {
            (dense_results, SearchMethod::Dense)
        } else if dense_results.is_empty() {
            (sparse_results.clone(), SearchMethod::Sparse)
        } else {
            (
                self.reciprocal_rank_fusion(&sparse_results, &dense_results),
                SearchMethod::Hybrid,
            )
        };

        let mut search_results: Vec<SearchResult> = results
            .into_iter()
            .filter_map(|(doc_id, score)| {
                self.documents.get(&doc_id).map(|doc| SearchResult {
                    doc_id,
                    content: doc.content.clone(),
                    source: doc.source.clone(),
                    score,
                    metadata: doc.metadata.clone(),
                    search_method: method.clone(),
                })
            })
            .filter(|r| r.score >= self.config.min_score)
            .take(self.config.max_results)
            .collect();

        if self.config.reranker_enabled && !search_results.is_empty() {
            search_results = Self::rerank(query, search_results)?;
        }

        Ok(search_results)
    }

    pub fn sparse_search(&self, query: &str) -> Vec<SearchResult> {
        let results = self.bm25_index.search(query, self.config.max_results);

        results
            .into_iter()
            .filter_map(|(doc_id, _source, score)| {
                self.documents.get(&doc_id).map(|doc| SearchResult {
                    doc_id,
                    content: doc.content.clone(),
                    source: doc.source.clone(),
                    score,
                    metadata: doc.metadata.clone(),
                    search_method: SearchMethod::Sparse,
                })
            })
            .collect()
    }

    pub async fn dense_search(
        &self,
        query_embedding: Vec<f32>,
    ) -> Result<Vec<SearchResult>, String> {
        let results = self
            .search_qdrant(&query_embedding, self.config.max_results)
            .await?;

        let search_results: Vec<SearchResult> = results
            .into_iter()
            .filter_map(|(doc_id, score)| {
                self.documents.get(&doc_id).map(|doc| SearchResult {
                    doc_id,
                    content: doc.content.clone(),
                    source: doc.source.clone(),
                    score,
                    metadata: doc.metadata.clone(),
                    search_method: SearchMethod::Dense,
                })
            })
            .collect();

        Ok(search_results)
    }

    pub(super) fn reciprocal_rank_fusion(
        &self,
        sparse: &[(String, f32)],
        dense: &[(String, f32)],
    ) -> Vec<(String, f32)> {
        let k = self.config.rrf_k as f32;
        let mut scores: HashMap<String, f32> = HashMap::new();

        for (rank, (doc_id, _)) in sparse.iter().enumerate() {
            let rrf_score = self.config.sparse_weight / (k + rank as f32 + 1.0);
            *scores.entry(doc_id.clone()).or_insert(0.0) += rrf_score;
        }

        for (rank, (doc_id, _)) in dense.iter().enumerate() {
            let rrf_score = self.config.dense_weight / (k + rank as f32 + 1.0);
            *scores.entry(doc_id.clone()).or_insert(0.0) += rrf_score;
        }

        let mut results: Vec<(String, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let max_score = results.first().map(|(_, s)| *s).unwrap_or(0.0);
        if max_score > 0.0 {
            for (_, score) in &mut results {
                *score /= max_score;
            }
        }

        results
    }

    pub(super) fn rerank(query: &str, results: Vec<SearchResult>) -> Result<Vec<SearchResult>, String> {
        let mut reranked = results;

        let query_lower = query.to_lowercase();
        let query_terms: std::collections::HashSet<String> = query_lower
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let query_terms_len = query_terms.len();

        for result in &mut reranked {
            let content_lower = result.content.to_lowercase();

            let mut overlap_score = 0.0;
            for term in &query_terms {
                if content_lower.contains(term) {
                    overlap_score += 1.0;
                }
            }

            let overlap_normalized = overlap_score / query_terms_len.max(1) as f32;
            result.score = result.score.mul_add(0.7, overlap_normalized * 0.3);
            result.search_method = SearchMethod::Reranked;
        }

        reranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(reranked)
    }

    async fn search_qdrant(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>, String> {
        let client = reqwest::Client::new();

        let search_request = serde_json::json!({
            "vector": embedding,
            "limit": limit,
            "with_payload": false
        });

        let response = client
            .post(format!(
                "{}/collections/{}/points/search",
                self.qdrant_url, self.collection_name
            ))
            .json(&search_request)
            .send()
            .await
            .map_err(|e| format!("Qdrant search failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Qdrant search error: {}", error_text));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Qdrant response: {}", e))?;

        let points = result["result"]
            .as_array()
            .ok_or("Invalid Qdrant response format")?;

        let results: Vec<(String, f32)> = points
            .iter()
            .filter_map(|p| {
                let id = p["id"].as_str().map(|s| s.to_string())?;
                let score = p["score"].as_f64()? as f32;
                Some((id, score))
            })
            .collect();

        Ok(results)
    }
}
