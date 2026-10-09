use super::*;

impl UpsertPointsBuilder {
    pub fn new(collection_name: &str, points: Vec<PointStruct>) -> Self {
        let points: Vec<Value> = points
            .into_iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "vector": p.vector,
                    "payload": p.payload
                })
            })
            .collect();

        Self {
            collection_name: collection_name.to_string(),
            points,
        }
    }

    pub async fn build(self, client: &QdrantClient) -> Result<()> {
        client.upsert_points(&self.collection_name, self.points).await
    }
}
