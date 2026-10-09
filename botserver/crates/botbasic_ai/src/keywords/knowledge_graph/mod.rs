pub(crate) use chrono::{DateTime, Utc};
pub(crate) use rhai::{Array, Dynamic, Engine, Map};
pub(crate) use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEntity {
    pub id: Uuid,

    pub bot_id: Uuid,

    pub entity_type: String,

    pub entity_name: String,

    pub aliases: Vec<String>,

    pub properties: serde_json::Value,

    pub confidence: f64,

    pub source: EntitySource,

    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum EntitySource {
    #[default]
    Manual,
    Extracted,
    Imported,
    Inferred,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgRelationship {
    pub id: Uuid,

    pub bot_id: Uuid,

    pub from_entity_id: Uuid,

    pub to_entity_id: Uuid,

    pub relationship_type: String,

    pub properties: serde_json::Value,

    pub confidence: f64,

    pub bidirectional: bool,

    pub source: EntitySource,

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,

    pub canonical_name: String,

    pub entity_type: String,

    pub start_pos: usize,

    pub end_pos: usize,

    pub confidence: f64,

    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    pub from_entity: String,

    pub to_entity: String,

    pub relationship_type: String,

    pub confidence: f64,

    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub entities: Vec<ExtractedEntity>,

    pub relationships: Vec<ExtractedRelationship>,

    pub metadata: ExtractionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    pub model: String,

    pub processing_time_ms: u64,

    pub tokens_processed: usize,

    pub text_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryResult {
    pub entities: Vec<KgEntity>,

    pub relationships: Vec<KgRelationship>,

    pub explanation: String,

    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct KnowledgeGraphConfig {
    pub enabled: bool,

    pub backend: String,

    pub extract_entities: bool,

    pub extraction_model: String,

    pub max_entities: usize,

    pub max_relationships: usize,

    pub min_confidence: f64,

    pub entity_types: Vec<String>,
}

#[derive(Debug)]
pub struct KnowledgeGraphManager {
    config: KnowledgeGraphConfig,
}

pub const KNOWLEDGE_GRAPH_SCHEMA: &str = r"
-- Knowledge graph entities
CREATE TABLE IF NOT EXISTS kg_entities (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL,
    entity_type VARCHAR(100) NOT NULL,
    entity_name VARCHAR(500) NOT NULL,
    aliases JSONB NOT NULL DEFAULT '[]',
    properties JSONB NOT NULL DEFAULT '{}',
    confidence DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    source VARCHAR(50) NOT NULL DEFAULT 'manual',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    UNIQUE(bot_id, entity_type, entity_name)
);

-- Knowledge graph relationships
CREATE TABLE IF NOT EXISTS kg_relationships (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL,
    from_entity_id UUID NOT NULL REFERENCES kg_entities(id) ON DELETE CASCADE,
    to_entity_id UUID NOT NULL REFERENCES kg_entities(id) ON DELETE CASCADE,
    relationship_type VARCHAR(100) NOT NULL,
    properties JSONB NOT NULL DEFAULT '{}',
    confidence DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    bidirectional BOOLEAN NOT NULL DEFAULT false,
    source VARCHAR(50) NOT NULL DEFAULT 'manual',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    UNIQUE(bot_id, from_entity_id, to_entity_id, relationship_type)
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_kg_entities_bot_id ON kg_entities(bot_id);
CREATE INDEX IF NOT EXISTS idx_kg_entities_type ON kg_entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_kg_entities_name ON kg_entities(entity_name);
CREATE INDEX IF NOT EXISTS idx_kg_entities_name_lower ON kg_entities(LOWER(entity_name));
CREATE INDEX IF NOT EXISTS idx_kg_entities_aliases ON kg_entities USING GIN(aliases);

CREATE INDEX IF NOT EXISTS idx_kg_relationships_bot_id ON kg_relationships(bot_id);
CREATE INDEX IF NOT EXISTS idx_kg_relationships_from ON kg_relationships(from_entity_id);
CREATE INDEX IF NOT EXISTS idx_kg_relationships_to ON kg_relationships(to_entity_id);
CREATE INDEX IF NOT EXISTS idx_kg_relationships_type ON kg_relationships(relationship_type);

-- Full-text search on entity names
CREATE INDEX IF NOT EXISTS idx_kg_entities_name_fts ON kg_entities
    USING GIN(to_tsvector('english', entity_name));
";

pub mod sql {
    pub const INSERT_ENTITY: &str = r"
        INSERT INTO kg_entities (
            id, bot_id, entity_type, entity_name, aliases, properties,
            confidence, source, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
        )
        ON CONFLICT (bot_id, entity_type, entity_name)
        DO UPDATE SET
            aliases = kg_entities.aliases || $5,
            properties = kg_entities.properties || $6,
            confidence = GREATEST(kg_entities.confidence, $7),
            updated_at = $10
        RETURNING id
    ";

    pub const INSERT_RELATIONSHIP: &str = r"
        INSERT INTO kg_relationships (
            id, bot_id, from_entity_id, to_entity_id, relationship_type,
            properties, confidence, bidirectional, source, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
        )
        ON CONFLICT (bot_id, from_entity_id, to_entity_id, relationship_type)
        DO UPDATE SET
            properties = kg_relationships.properties || $6,
            confidence = GREATEST(kg_relationships.confidence, $7)
        RETURNING id
    ";

    pub const GET_ENTITY_BY_NAME: &str = r"
        SELECT * FROM kg_entities
        WHERE bot_id = $1
        AND (
            LOWER(entity_name) = LOWER($2)
            OR aliases @> $3::jsonb
        )
        LIMIT 1
    ";

    pub const GET_ENTITY_BY_ID: &str = r"
        SELECT * FROM kg_entities WHERE id = $1
    ";

    pub const SEARCH_ENTITIES: &str = r"
        SELECT * FROM kg_entities
        WHERE bot_id = $1
        AND (
            to_tsvector('english', entity_name) @@ plainto_tsquery('english', $2)
            OR LOWER(entity_name) LIKE LOWER($3)
        )
        ORDER BY confidence DESC
        LIMIT $4
    ";

    pub const GET_ENTITIES_BY_TYPE: &str = r"
        SELECT * FROM kg_entities
        WHERE bot_id = $1 AND entity_type = $2
        ORDER BY entity_name
        LIMIT $3
    ";

    pub const GET_RELATED_ENTITIES: &str = r"
        SELECT e.*, r.relationship_type, r.confidence as rel_confidence
        FROM kg_entities e
        JOIN kg_relationships r ON (
            (r.from_entity_id = $1 AND r.to_entity_id = e.id)
            OR (r.bidirectional AND r.to_entity_id = $1 AND r.from_entity_id = e.id)
        )
        WHERE r.bot_id = $2
        ORDER BY r.confidence DESC
        LIMIT $3
    ";

    pub const GET_RELATED_BY_TYPE: &str = r"
        SELECT e.*, r.relationship_type, r.confidence as rel_confidence
        FROM kg_entities e
        JOIN kg_relationships r ON (
            (r.from_entity_id = $1 AND r.to_entity_id = e.id)
            OR (r.bidirectional AND r.to_entity_id = $1 AND r.from_entity_id = e.id)
        )
        WHERE r.bot_id = $2 AND r.relationship_type = $3
        ORDER BY r.confidence DESC
        LIMIT $4
    ";

    pub const GET_RELATIONSHIP: &str = r"
        SELECT * FROM kg_relationships
        WHERE bot_id = $1
        AND from_entity_id = $2
        AND to_entity_id = $3
        AND relationship_type = $4
    ";

    pub const GET_ALL_RELATIONSHIPS_FOR_ENTITY: &str = r"
        SELECT r.*,
               e1.entity_name as from_name, e1.entity_type as from_type,
               e2.entity_name as to_name, e2.entity_type as to_type
        FROM kg_relationships r
        JOIN kg_entities e1 ON r.from_entity_id = e1.id
        JOIN kg_entities e2 ON r.to_entity_id = e2.id
        WHERE r.bot_id = $1
        AND (r.from_entity_id = $2 OR r.to_entity_id = $2)
        ORDER BY r.confidence DESC
    ";

    pub const DELETE_ENTITY: &str = r"
        DELETE FROM kg_entities WHERE id = $1 AND bot_id = $2
    ";

    pub const DELETE_RELATIONSHIP: &str = r"
        DELETE FROM kg_relationships WHERE id = $1 AND bot_id = $2
    ";

    pub const COUNT_ENTITIES: &str = r"
        SELECT COUNT(*) FROM kg_entities WHERE bot_id = $1
    ";

    pub const COUNT_RELATIONSHIPS: &str = r"
        SELECT COUNT(*) FROM kg_relationships WHERE bot_id = $1
    ";

    pub const GET_ENTITY_TYPES: &str = r"
        SELECT DISTINCT entity_type, COUNT(*) as count
        FROM kg_entities
        WHERE bot_id = $1
        GROUP BY entity_type
        ORDER BY count DESC
    ";

    pub const GET_RELATIONSHIP_TYPES: &str = r"
        SELECT DISTINCT relationship_type, COUNT(*) as count
        FROM kg_relationships
        WHERE bot_id = $1
        GROUP BY relationship_type
        ORDER BY count DESC
    ";

    pub const FIND_PATH: &str = r"
        WITH RECURSIVE path_finder AS (
            -- Base case: start from source entity
            SELECT
                from_entity_id,
                to_entity_id,
                relationship_type,
                ARRAY[from_entity_id] as path,
                1 as depth
            FROM kg_relationships
            WHERE bot_id = $1 AND from_entity_id = $2

            UNION ALL

            -- Recursive case: follow relationships
            SELECT
                r.from_entity_id,
                r.to_entity_id,
                r.relationship_type,
                pf.path || r.from_entity_id,
                pf.depth + 1
            FROM kg_relationships r
            JOIN path_finder pf ON r.from_entity_id = pf.to_entity_id
            WHERE r.bot_id = $1
            AND NOT r.from_entity_id = ANY(pf.path)  -- Prevent cycles
            AND pf.depth < $4  -- Max depth
        )
        SELECT * FROM path_finder
        WHERE to_entity_id = $3
        ORDER BY depth
        LIMIT 1
    ";
}

pub mod relationship_types {
    pub const WORKS_ON: &str = "works_on";
    pub const REPORTS_TO: &str = "reports_to";
    pub const MANAGES: &str = "manages";
    pub const OWNS: &str = "owns";
    pub const PART_OF: &str = "part_of";
    pub const LOCATED_IN: &str = "located_in";
    pub const RELATED_TO: &str = "related_to";
    pub const CREATED_BY: &str = "created_by";
    pub const DEPENDS_ON: &str = "depends_on";
    pub const CONNECTED_TO: &str = "connected_to";
    pub const MEMBER_OF: &str = "member_of";
    pub const SUCCESSOR_OF: &str = "successor_of";
    pub const PREDECESSOR_OF: &str = "predecessor_of";
    pub const ALIAS_OF: &str = "alias_of";
}

pub mod entity_types {
    pub const PERSON: &str = "person";
    pub const ORGANIZATION: &str = "organization";
    pub const PROJECT: &str = "project";
    pub const PRODUCT: &str = "product";
    pub const LOCATION: &str = "location";
    pub const EVENT: &str = "event";
    pub const CONCEPT: &str = "concept";
    pub const DOCUMENT: &str = "document";
    pub const TEAM: &str = "team";
    pub const ROLE: &str = "role";
    pub const SKILL: &str = "skill";
    pub const TECHNOLOGY: &str = "technology";
}

use std::collections::HashMap;
use log::info;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;

pub use part3::*;
