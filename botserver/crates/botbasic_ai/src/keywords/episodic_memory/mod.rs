pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use rhai::{Array, Dynamic, Engine, Map};
pub(crate) use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: Uuid,

    pub user_id: Uuid,

    pub bot_id: Uuid,

    pub session_id: Uuid,

    pub summary: String,

    pub key_topics: Vec<String>,

    pub decisions: Vec<String>,

    pub action_items: Vec<ActionItem>,

    pub sentiment: Sentiment,

    pub resolution: ResolutionStatus,

    pub message_count: usize,

    pub message_ids: Vec<Uuid>,

    pub created_at: DateTime<Utc>,

    pub conversation_start: DateTime<Utc>,
    pub conversation_end: DateTime<Utc>,

    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub description: String,

    pub assignee: Option<String>,

    pub due_date: Option<DateTime<Utc>>,

    pub priority: Priority,

    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentiment {
    pub score: f64,

    pub label: SentimentLabel,

    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SentimentLabel {
    VeryNegative,
    Negative,
    #[default]
    Neutral,
    Positive,
    VeryPositive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ResolutionStatus {
    Resolved,
    Unresolved,
    Escalated,
    Pending,
    #[default]
    Unknown,
}


#[derive(Debug, Clone)]
pub struct EpisodicMemoryConfig {
    pub enabled: bool,

    pub threshold: usize,

    pub history: usize,

    pub model: String,

    pub max_episodes: usize,

    pub retention_days: u32,

    pub auto_summarize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug)]
pub struct EpisodicMemoryManager {
    config: EpisodicMemoryConfig,
}

pub const EPISODIC_MEMORY_SCHEMA: &str = r#"
-- Conversation episodes (summaries)
CREATE TABLE IF NOT EXISTS conversation_episodes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    bot_id UUID NOT NULL,
    session_id UUID NOT NULL,
    summary TEXT NOT NULL,
    key_topics JSONB NOT NULL DEFAULT '[]',
    decisions JSONB NOT NULL DEFAULT '[]',
    action_items JSONB NOT NULL DEFAULT '[]',
    sentiment JSONB NOT NULL DEFAULT '{"score": 0, "label": "neutral", "confidence": 0.5}',
    resolution VARCHAR(50) NOT NULL DEFAULT 'unknown',
    message_count INTEGER NOT NULL DEFAULT 0,
    message_ids JSONB NOT NULL DEFAULT '[]',
    conversation_start TIMESTAMP WITH TIME ZONE NOT NULL,
    conversation_end TIMESTAMP WITH TIME ZONE NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_episodes_user_id ON conversation_episodes(user_id);
CREATE INDEX IF NOT EXISTS idx_episodes_bot_id ON conversation_episodes(bot_id);
CREATE INDEX IF NOT EXISTS idx_episodes_session_id ON conversation_episodes(session_id);
CREATE INDEX IF NOT EXISTS idx_episodes_created_at ON conversation_episodes(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_episodes_key_topics ON conversation_episodes USING GIN(key_topics);
CREATE INDEX IF NOT EXISTS idx_episodes_resolution ON conversation_episodes(resolution);

-- Full-text search on summaries
CREATE INDEX IF NOT EXISTS idx_episodes_summary_fts ON conversation_episodes
    USING GIN(to_tsvector('english', summary));
"#;

pub mod sql {
    pub const INSERT_EPISODE: &str = r"
        INSERT INTO conversation_episodes (
            id, user_id, bot_id, session_id, summary, key_topics, decisions,
            action_items, sentiment, resolution, message_count, message_ids,
            conversation_start, conversation_end, metadata, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16
        )
    ";

    pub const GET_EPISODES_BY_USER: &str = r"
        SELECT * FROM conversation_episodes
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
    ";

    pub const GET_EPISODES_BY_SESSION: &str = r"
        SELECT * FROM conversation_episodes
        WHERE session_id = $1
        ORDER BY created_at DESC
    ";

    pub const SEARCH_EPISODES: &str = r"
        SELECT * FROM conversation_episodes
        WHERE user_id = $1
        AND (
            to_tsvector('english', summary) @@ plainto_tsquery('english', $2)
            OR key_topics @> $3::jsonb
        )
        ORDER BY created_at DESC
        LIMIT $4
    ";

    pub const DELETE_OLD_EPISODES: &str = r"
        DELETE FROM conversation_episodes
        WHERE created_at < $1
    ";

    pub const COUNT_USER_EPISODES: &str = r"
        SELECT COUNT(*) FROM conversation_episodes
        WHERE user_id = $1
    ";

    pub const DELETE_OLDEST_EPISODES: &str = r"
        DELETE FROM conversation_episodes
        WHERE id IN (
            SELECT id FROM conversation_episodes
            WHERE user_id = $1
            ORDER BY created_at ASC
            LIMIT $2
        )
    ";
}

use log::info;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;

pub use part3::*;
pub use part4::*;
