use super::*;

impl SearchPointsBuilder {
    pub fn new(collection_name: &str, vector: Vec<f32>, limit: usize) -> Self {
        Self {
            collection_name: collection_name.to_string(),
            vector,
            limit,
            filter: None,
        }
    }

    pub fn filter(mut self, filter: Option<Filter>) -> Self {
        self.filter = filter.map(|f| json!(f));
        self
    }

    pub fn with_payload(self, _with: bool) -> Self {
        self
    }

    pub async fn build(self, client: &QdrantClient) -> Result<SearchResponse> {
        let points = client
            .search_points(&self.collection_name, &self.vector, self.limit, self.filter)
            .await?;
        
        Ok(SearchResponse {
            result: points,
        })
    }
}
