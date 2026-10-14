pub(crate) use crate::config::ConfigManager;
pub(crate) use crate::kb::web_crawler::{WebCrawler, WebsiteCrawlConfig};
pub(crate) use crate::kb::kb_indexer::{KbIndexer, QdrantConfig};
pub(crate) use log::{info, trace, warn};
pub(crate) use tokio::time::{interval, Duration};

#[derive(Debug)]
pub struct WebsiteCrawlerService {
    db_pool: DbPool,
    check_interval: Duration,
    running: Arc<tokio::sync::RwLock<bool>>,
    active_crawls: Arc<tokio::sync::RwLock<HashSet<String>>>,
}

#[derive(QueryableByName)]
struct CountResult {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName, Debug)]
pub struct WebsiteCrawlRecord {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub bot_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub url: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub expires_policy: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub refresh_policy: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub max_depth: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub max_pages: i32,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub next_crawl: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::SmallInt>)]
    pub crawl_status: Option<i16>,
}

/// Request payload for force recrawl endpoint
#[derive(serde::Deserialize)]
pub struct ForceRecrawlRequest {
    pub bot_id: uuid::Uuid,
    pub url: String,
}

use crate::kb::embedding_generator::EmbeddingConfig;
use crate::shared::state::AppState;
use crate::shared::utils::DbPool;
use diesel::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;

pub use part3::*;
pub use part4::*;
