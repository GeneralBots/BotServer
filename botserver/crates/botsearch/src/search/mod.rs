pub(crate) use botlib::db_pool::DbPool;
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::{BigInt, Float, Integer, Nullable, Text, Timestamptz};
pub(crate) use log::{debug, info};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSource {
    Email,
    Drive,
    Calendar,
    Tasks,
    Transcription,
    Chat,
    Contacts,
    Notes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub source: SearchSource,
    pub title: String,
    pub snippet: String,
    pub url: Option<String>,
    pub score: f32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub sources: Option<Vec<SearchSource>>,
    pub organization_id: Uuid,
    pub user_id: Option<Uuid>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: i64,
    pub query_time_ms: u64,
    pub sources_searched: Vec<SearchSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub source: SearchSource,
    pub document_count: i64,
    pub last_indexed: Option<DateTime<Utc>>,
    pub index_size_bytes: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchConfig {
    pub enabled_sources: Vec<SearchSource>,
    pub max_results_per_source: i32,
    pub snippet_length: i32,
    pub use_tantivy: bool,
    pub postgres_fts_config: String,
    pub index_retention_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentToIndex {
    pub id: String,
    pub source: SearchSource,
    pub organization_id: Uuid,
    pub title: String,
    pub content: String,
    pub url: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

#[derive(QueryableByName, Debug)]
struct FtsSearchRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    source: String,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    snippet: String,
    #[diesel(sql_type = Nullable<Text>)]
    url: Option<String>,
    #[diesel(sql_type = Float)]
    score: f32,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    created_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    updated_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Text>)]
    metadata_json: Option<String>,
}

#[derive(QueryableByName, Debug)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName, Debug)]
struct IndexStatsRow {
    #[diesel(sql_type = Text)]
    source: String,
    #[diesel(sql_type = BigInt)]
    document_count: i64,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    last_indexed: Option<DateTime<Utc>>,
    #[diesel(sql_type = BigInt)]
    index_size_bytes: i64,
}

#[derive(QueryableByName, Debug)]
pub struct CrmPersonMatch {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub subtitle: String,
}

pub struct SearchService {
    pool: Arc<DbPool>,
    config: SearchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    pub success_count: i32,
    pub error_count: i32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum SearchError {
    DatabaseConnection,
    QueryFailed(String),
    IndexFailed(String),
    DeleteFailed(String),
    InvalidQuery(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_source_display() {
        assert_eq!(SearchSource::Email.to_string(), "email");
        assert_eq!(SearchSource::Drive.to_string(), "drive");
        assert_eq!(SearchSource::Calendar.to_string(), "calendar");
        assert_eq!(SearchSource::Tasks.to_string(), "tasks");
        assert_eq!(SearchSource::Transcription.to_string(), "transcription");
    }

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default_config();
        assert!(!config.enabled_sources.is_empty());
        assert_eq!(config.max_results_per_source, 50);
        assert_eq!(config.snippet_length, 200);
        assert!(!config.use_tantivy);
    }

    #[test]
    fn test_search_error_display() {
        let err = SearchError::DatabaseConnection;
        assert_eq!(err.to_string(), "Database connection failed");

        let err = SearchError::QueryFailed("test error".to_string());
        assert_eq!(err.to_string(), "Query failed: test error");
    }

    #[test]
    fn test_document_to_index_creation() {
        let doc = DocumentToIndex {
            id: "test-123".to_string(),
            source: SearchSource::Email,
            organization_id: Uuid::new_v4(),
            title: "Test Email".to_string(),
            content: "This is test content".to_string(),
            url: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        assert_eq!(doc.source, SearchSource::Email);
        assert_eq!(doc.title, "Test Email");
    }

    #[test]
    fn test_index_result() {
        let result = IndexResult {
            success_count: 10,
            error_count: 2,
            errors: vec!["error1".to_string(), "error2".to_string()],
        };
        assert_eq!(result.success_count, 10);
        assert_eq!(result.error_count, 2);
        assert_eq!(result.errors.len(), 2);
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

pub use part6::*;
