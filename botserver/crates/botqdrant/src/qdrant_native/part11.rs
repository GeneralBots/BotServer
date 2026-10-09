use super::*;

impl DeletePointsBuilder {
    pub fn new(collection_name: &str) -> Self {
        Self {
            collection_name: collection_name.to_string(),
            points: Vec::new(),
        }
    }

    pub fn points(mut self, point_ids: Vec<PointId>) -> Self {
        self.points = point_ids;
        self
    }

    pub async fn build(self, client: &QdrantClient) -> Result<()> {
        let point_ids: Vec<String> = self.points.into_iter().map(|p| p.id).collect();
        client.delete_points(&self.collection_name, point_ids).await
    }
}
