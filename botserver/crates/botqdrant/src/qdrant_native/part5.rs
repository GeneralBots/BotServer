use super::*;

impl PointStruct {
    pub fn new(id: String, vector: Vec<f32>, payload: serde_json::Map<String, Value>) -> Self {
        Self { id, vector, payload }
    }
}
