pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::hash::{Hash, Hasher};
pub(crate) use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CacheRegion {
    Users,
    Organizations,
    Bots,
    Sessions,
    Permissions,
    KnowledgeBase,
    Conversations,
    Settings,
    Metrics,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub default_ttl_seconds: u64,
    pub cleanup_interval_seconds: u64,
    pub enable_statistics: bool,
    pub compression_threshold_bytes: Option<usize>,
    pub region_configs: HashMap<CacheRegion, RegionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    pub max_entries: usize,
    pub ttl_seconds: u64,
    pub eviction_policy: EvictionPolicy,
    pub preload: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    Fifo,
    Ttl,
    Random,
}

#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    pub value: V,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub size_bytes: usize,
}

#[derive(Debug, Default)]
pub struct CacheStatistics {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
    pub expirations: AtomicU64,
    pub total_size_bytes: AtomicU64,
    pub entry_count: AtomicU64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatisticsSummary {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub evictions: u64,
    pub expirations: u64,
    pub total_size_bytes: u64,
    pub entry_count: u64,
}

pub struct PerformanceCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    config: CacheConfig,
    statistics: Arc<CacheStatistics>,
    region: CacheRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptimizationHint {
    pub use_index: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<Vec<(String, SortDirection)>>,
    pub select_fields: Option<Vec<String>>,
    pub include_count: bool,
    pub use_cache: bool,
    pub cache_ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub estimated_rows: u64,
    pub estimated_cost: f64,
    pub index_usage: Vec<IndexUsage>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<QuerySuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexUsage {
    pub index_name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_covering: bool,
    pub selectivity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySuggestion {
    pub suggestion_type: PerformanceSuggestionType,
    pub description: String,
    pub impact: Impact,
    pub implementation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceSuggestionType {
    AddIndex,
    UseIndex,
    AddLimit,
    RemoveSelectStar,
    UseJoinInsteadOfSubquery,
    AddCaching,
    Denormalize,
    Partition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Impact {
    High,
    Medium,
    Low,
}

pub struct QueryOptimizer {
    slow_query_threshold_ms: u64,
    query_history: Arc<RwLock<Vec<QueryMetrics>>>,
    table_statistics: Arc<RwLock<HashMap<String, TableStats>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    pub query_hash: String,
    pub query_pattern: String,
    pub execution_count: u64,
    pub total_time_ms: u64,
    pub avg_time_ms: f64,
    pub max_time_ms: u64,
    pub min_time_ms: u64,
    pub rows_examined: u64,
    pub rows_returned: u64,
    pub last_executed: DateTime<Utc>,
    pub is_slow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub table_name: String,
    pub row_count: u64,
    pub avg_row_size_bytes: u64,
    pub index_count: u32,
    pub last_analyzed: DateTime<Utc>,
    pub hot_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub generated_at: DateTime<Utc>,
    pub total_queries: u64,
    pub slow_queries: u64,
    pub slow_query_percentage: f64,
    pub average_query_time_ms: f64,
    pub top_slow_queries: Vec<QueryMetrics>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
    pub test_on_checkout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolMetrics {
    pub active_connections: u32,
    pub idle_connections: u32,
    pub waiting_requests: u32,
    pub total_connections_created: u64,
    pub total_connections_closed: u64,
    pub average_wait_time_ms: f64,
    pub pool_utilization: f64,
}

type BatchProcessorFunc<T> = Arc<dyn Fn(Vec<T>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;

pub struct BatchProcessor<T> {
    batch_size: usize,
    buffer: Arc<RwLock<Vec<T>>>,
    processor: BatchProcessorFunc<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub default_page_size: u32,
    pub max_page_size: u32,
    pub use_cursor_pagination: bool,
    pub include_total_count: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub page_info: PageInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
    pub total_count: Option<u64>,
    pub page_size: u32,
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;

pub use part10::*;
