

pub mod ui;
pub mod web_search;

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Form, Json, Router,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use botlib::db_pool::DbPool;
use botlib::traits::LLMProvider;

pub const ROUTE_RESEARCH_COLLECTIONS: &str = "/api/ui/research/collections";
pub const ROUTE_RESEARCH_COLLECTIONS_NEW: &str = "/api/ui/research/collections/new";
pub const ROUTE_RESEARCH_COLLECTION_BY_ID: &str = "/api/ui/research/collections/{id}";
pub const ROUTE_RESEARCH_SEARCH: &str = "/api/ui/research/search";
pub const ROUTE_RESEARCH_RECENT: &str = "/api/ui/research/recent";
pub const ROUTE_RESEARCH_TRENDING: &str = "/api/ui/research/trending";
pub const ROUTE_RESEARCH_PROMPTS: &str = "/api/ui/research/prompts";
pub const ROUTE_RESEARCH_EXPORT_CITATIONS: &str = "/api/ui/research/export/citations";
pub const ROUTE_RESEARCH_SOURCE_COUNTS: &str = "/api/ui/research/source-counts";
pub const ROUTE_RESEARCH_SOURCES: &str = "/api/ui/research/sources";
pub const ROUTE_RESEARCH_COLLECTIONS_SAVE: &str = "/api/ui/research/collections/save";
pub const ROUTE_RESEARCH_SHARE: &str = "/api/ui/research/share";

pub trait ResearchState: Send + Sync + std::fmt::Debug + 'static {
    fn db_pool(&self) -> &DbPool;
    fn llm_provider(&self) -> Option<Arc<dyn LLMProvider>>;
    fn bot_id(&self) -> Option<uuid::Uuid>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub collection: Option<String>,
    pub filters: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub collection: Option<String>,
    pub filters: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCollectionRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct KbDocumentRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub file_path: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub collection_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Jsonb>)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CollectionRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub folder_path: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub document_count: i64,
}

#[derive(Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RecentSearchRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub query: String,
}

#[derive(Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TrendingTagRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub query: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub n: i64,
}

#[derive(Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct SourceCountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub n: i64,
}

#[derive(Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CollectionNameRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
}

mod lib_part1;
mod lib_part2;
mod lib_part3;
mod lib_part4;
mod lib_part5;

pub use lib_part1::*;
pub use lib_part2::*;
pub use lib_part3::*;
pub use lib_part4::*;
pub use lib_part5::*;
