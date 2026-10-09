use super::*;

impl KbIndexer {
    pub async fn get_collection_info(&self, collection_name: &str) -> Result<CollectionInfo> {
        let info_url = format!("{}/collections/{}", self.qdrant_config.url, collection_name);

        let response = self.http_client.get(&info_url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 {
                return Ok(CollectionInfo {
                    name: collection_name.to_string(),
                    points_count: 0,
                    vectors_count: 0,
                    indexed_vectors_count: 0,
                    segments_count: 0,
                    status: "not_found".to_string(),
                });
            }
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to get collection info: {}",
                error_text
            ));
        }

        let response_json: serde_json::Value = response.json().await?;

        let result = &response_json["result"];

        let points_count = result["points_count"].as_u64().unwrap_or(0) as usize;
        let vectors_count = result["vectors_count"]
            .as_u64()
            .or_else(|| {
                result["vectors_count"]
                    .as_object()
                    .map(|_| points_count as u64)
            })
            .unwrap_or(0) as usize;
        let indexed_vectors_count = result["indexed_vectors_count"]
            .as_u64()
            .unwrap_or(vectors_count as u64) as usize;
        let segments_count = result["segments_count"].as_u64().unwrap_or(0) as usize;
        let status = result["status"].as_str().unwrap_or("unknown").to_string();

        Ok(CollectionInfo {
            name: collection_name.to_string(),
            points_count,
            vectors_count,
            indexed_vectors_count,
            segments_count,
            status,
        })
    }
}
