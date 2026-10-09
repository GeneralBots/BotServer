use super::*;

impl CreateCollectionBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            vector_size: 0,
            distance: "Cosine".to_string(),
        }
    }

    pub fn vector(mut self, params: VectorParamsBuilder) -> Self {
        self.vector_size = params.size;
        self.distance = params.distance;
        self
    }

    pub fn vectors_config(mut self, params: VectorParams) -> Self {
        self.vector_size = params.size;
        self.distance = format!("{:?}", params.distance);
        self
    }

    pub async fn build(self, client: &QdrantClient) -> Result<()> {
        client
            .create_collection(&self.name, self.vector_size, &self.distance)
            .await
    }
}
