use super::*;

impl KnowledgeGraphManager {
    pub fn new(config: KnowledgeGraphConfig) -> Self {
        Self { config }
    }

    pub fn from_config(config_map: &HashMap<String, String>) -> Self {
        let config = KnowledgeGraphConfig {
            enabled: config_map
                .get("knowledge-graph-enabled")
                .map(|v| v == "true")
                .unwrap_or(true),
            backend: config_map
                .get("knowledge-graph-backend")
                .cloned()
                .unwrap_or_else(|| "postgresql".to_string()),
            extract_entities: config_map
                .get("knowledge-graph-extract-entities")
                .map(|v| v == "true")
                .unwrap_or(true),
            extraction_model: config_map
                .get("knowledge-graph-extraction-model")
                .cloned()
                .unwrap_or_else(|| "quality".to_string()),
            max_entities: config_map
                .get("knowledge-graph-max-entities")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10000),
            max_relationships: config_map
                .get("knowledge-graph-max-relationships")
                .and_then(|v| v.parse().ok())
                .unwrap_or(50000),
            min_confidence: config_map
                .get("knowledge-graph-min-confidence")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.7),
            entity_types: config_map
                .get("knowledge-graph-entity-types")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| KnowledgeGraphConfig::default().entity_types),
        };
        Self::new(config)
    }

    pub fn generate_extraction_prompt(&self, text: &str) -> String {
        let entity_types = self.config.entity_types.join(", ");

        format!(
            r#"Extract entities and relationships from the following text.

ENTITY TYPES TO EXTRACT: {entity_types}

TEXT:
{text}

Respond with valid JSON only:
{{
    "entities": [
        {{
            "name": "exact name as in text",
            "canonical_name": "normalized name",
            "entity_type": "person|organization|project|product|location|event|concept",
            "confidence": 0.95,
            "properties": {{"key": "value"}}
        }}
    ],
    "relationships": [
        {{
            "from_entity": "entity name",
            "to_entity": "entity name",
            "relationship_type": "works_on|reports_to|owns|part_of|located_in|related_to",
            "confidence": 0.9,
            "evidence": "text snippet supporting this relationship"
        }}
    ]
}}"#
        )
    }

    pub fn generate_query_prompt(&self, query: &str, context: &str) -> String {
        format!(
            r"Answer this question using the knowledge graph context.

QUESTION: {query}

KNOWLEDGE GRAPH CONTEXT:
{context}

Provide a natural language answer based on the entities and relationships.
If the information is not available, say so clearly.
"
        )
    }

    pub fn parse_extraction_response(
        &self,
        response: &str,
        text_length: usize,
        processing_time_ms: u64,
    ) -> Result<ExtractionResult, String> {
        let json_str = extract_json(response)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let entities: Vec<ExtractedEntity> = parsed["entities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(ExtractedEntity {
                            name: v["name"].as_str()?.to_string(),
                            canonical_name: v["canonical_name"]
                                .as_str()
                                .unwrap_or(v["name"].as_str()?)
                                .to_string(),
                            entity_type: v["entity_type"].as_str()?.to_string(),
                            start_pos: 0,
                            end_pos: 0,
                            confidence: v["confidence"].as_f64().unwrap_or(0.8),
                            properties: v["properties"].clone(),
                        })
                    })
                    .filter(|e| e.confidence >= self.config.min_confidence)
                    .collect()
            })
            .unwrap_or_default();

        let relationships: Vec<ExtractedRelationship> = parsed["relationships"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(ExtractedRelationship {
                            from_entity: v["from_entity"].as_str()?.to_string(),
                            to_entity: v["to_entity"].as_str()?.to_string(),
                            relationship_type: v["relationship_type"].as_str()?.to_string(),
                            confidence: v["confidence"].as_f64().unwrap_or(0.8),
                            evidence: v["evidence"].as_str().unwrap_or("").to_string(),
                        })
                    })
                    .filter(|r| r.confidence >= self.config.min_confidence)
                    .collect()
            })
            .unwrap_or_default();

        Ok(ExtractionResult {
            entities,
            relationships,
            metadata: ExtractionMetadata {
                model: self.config.extraction_model.clone(),
                processing_time_ms,
                tokens_processed: text_length / 4,
                text_length,
            },
        })
    }

    pub fn should_extract(&self) -> bool {
        self.config.enabled && self.config.extract_entities
    }

    pub fn is_valid_entity_type(&self, entity_type: &str) -> bool {
        self.config
            .entity_types
            .iter()
            .any(|t| t.eq_ignore_ascii_case(entity_type))
    }
}
pub(super) fn extract_json(response: &str) -> Result<String, String> {
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start + 7..].find("```") {
            return Ok(response[start + 7..start + 7 + end].trim().to_string());
        }
    }

    if let Some(start) = response.find("```") {
        let after_start = start + 3;
        let json_start = response[after_start..]
            .find('\n')
            .map(|i| after_start + i + 1)
            .unwrap_or(after_start);
        if let Some(end) = response[json_start..].find("```") {
            return Ok(response[json_start..json_start + end].trim().to_string());
        }
    }

    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return Ok(response[start..=end].to_string());
            }
        }
    }

    Err("No JSON found in response".to_string())
}
impl KgEntity {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();

        map.insert("id".into(), self.id.to_string().into());
        map.insert("bot_id".into(), self.bot_id.to_string().into());
        map.insert("entity_type".into(), self.entity_type.clone().into());
        map.insert("entity_name".into(), self.entity_name.clone().into());

        let aliases: Array = self
            .aliases
            .iter()
            .map(|a| Dynamic::from(a.clone()))
            .collect();
        map.insert("aliases".into(), aliases.into());

        map.insert("properties".into(), json_to_dynamic(&self.properties));
        map.insert("confidence".into(), self.confidence.into());
        map.insert(
            "source".into(),
            format!("{:?}", self.source).to_lowercase().into(),
        );
        map.insert("created_at".into(), self.created_at.to_rfc3339().into());
        map.insert("updated_at".into(), self.updated_at.to_rfc3339().into());

        Dynamic::from(map)
    }
}
impl KgRelationship {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();

        map.insert("id".into(), self.id.to_string().into());
        map.insert("bot_id".into(), self.bot_id.to_string().into());
        map.insert(
            "from_entity_id".into(),
            self.from_entity_id.to_string().into(),
        );
        map.insert("to_entity_id".into(), self.to_entity_id.to_string().into());
        map.insert(
            "relationship_type".into(),
            self.relationship_type.clone().into(),
        );
        map.insert("properties".into(), json_to_dynamic(&self.properties));
        map.insert("confidence".into(), self.confidence.into());
        map.insert("bidirectional".into(), self.bidirectional.into());
        map.insert(
            "source".into(),
            format!("{:?}", self.source).to_lowercase().into(),
        );
        map.insert("created_at".into(), self.created_at.to_rfc3339().into());

        Dynamic::from(map)
    }
}
pub(super) fn json_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let array: Array = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(array)
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}
