use super::*;

impl Default for KnowledgeGraphConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "postgresql".to_string(),
            extract_entities: true,
            extraction_model: "quality".to_string(),
            max_entities: 10000,
            max_relationships: 50000,
            min_confidence: 0.7,
            entity_types: vec![
                "person".to_string(),
                "organization".to_string(),
                "project".to_string(),
                "product".to_string(),
                "location".to_string(),
                "event".to_string(),
                "concept".to_string(),
            ],
        }
    }
}
