use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text, Timestamptz};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use std::sync::Arc;
use uuid::Uuid;

pub type DbPool = diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;

const DEFAULT_RETENTION_DAYS: i64 = 180;
const MIN_RETENTION_DAYS: i64 = 7;
const MAX_RETENTION_DAYS: i64 = 3650;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupCategory {
    BotHistory,
    ChatSessions,
    SystemLogs,
    AccessLogs,
    DebugLogs,
    TempFiles,
    CacheData,
    AnalyticsRaw,
    OldCalendarEvents,
    Attachments,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupConfig {
    pub organization_id: Uuid,
    pub retention_days: i64,
    pub enabled_categories: Vec<CleanupCategory>,
    pub auto_cleanup_enabled: bool,
    pub auto_cleanup_schedule: Option<String>,
    pub storage_alert_threshold_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPreview {
    pub categories: Vec<CategoryPreview>,
    pub total_records: i64,
    pub estimated_bytes_freed: i64,
    pub estimated_bytes_freed_formatted: String,
    pub cutoff_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPreview {
    pub category: CleanupCategory,
    pub display_name: String,
    pub record_count: i64,
    pub estimated_bytes: i64,
    pub estimated_bytes_formatted: String,
    pub oldest_record: Option<DateTime<Utc>>,
    pub is_essential: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub success: bool,
    pub categories_cleaned: Vec<CategoryResult>,
    pub total_records_deleted: i64,
    pub bytes_freed: i64,
    pub bytes_freed_formatted: String,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryResult {
    pub category: CleanupCategory,
    pub display_name: String,
    pub records_deleted: i64,
    pub bytes_freed: i64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub organization_id: Uuid,
    pub categories: Vec<CategoryStorage>,
    pub total_bytes: i64,
    pub total_bytes_formatted: String,
    pub quota_bytes: Option<i64>,
    pub quota_percent_used: Option<f32>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStorage {
    pub category: CleanupCategory,
    pub display_name: String,
    pub bytes_used: i64,
    pub bytes_formatted: String,
    pub record_count: i64,
    pub percent_of_total: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupHistory {
    pub id: String,
    pub organization_id: Uuid,
    pub performed_at: DateTime<Utc>,
    pub performed_by: Option<Uuid>,
    pub trigger: CleanupTrigger,
    pub categories_cleaned: Vec<CleanupCategory>,
    pub records_deleted: i64,
    pub bytes_freed: i64,
    pub retention_days_used: i64,
    pub duration_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupTrigger {
    Manual,
    Scheduled,
    StorageAlert,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct SizeRow {
    #[diesel(sql_type = BigInt)]
    size_bytes: i64,
}

#[derive(QueryableByName)]
struct OldestRow {
    #[diesel(sql_type = Timestamptz)]
    oldest: DateTime<Utc>,
}

pub struct CleanupService {
    pool: Arc<DbPool>,
}

#[derive(Debug, Clone)]
pub enum CleanupError {
    DatabaseConnection,
    CleanupFailed(String),
    ConfigSaveFailed,
    HistorySaveFailed,
    OrganizationNotFound,
    RetentionTooShort(i64),
    RetentionTooLong(i64),
    InvalidCategory(String),
}

#[derive(Debug, Deserialize)]
struct PreviewQuery {
    organization_id: Uuid,
    retention_days: Option<i64>,
    categories: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExecuteRequest {
    organization_id: Uuid,
    retention_days: Option<i64>,
    categories: Option<Vec<String>>,
    performed_by: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct StorageQuery {
    organization_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct HistoryQuery {
    organization_id: Uuid,
    limit: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_category_display() {
        assert_eq!(CleanupCategory::BotHistory.to_string(), "Bot History");
        assert_eq!(CleanupCategory::ChatSessions.to_string(), "Chat Sessions");
        assert_eq!(CleanupCategory::SystemLogs.to_string(), "System Logs");
    }

    #[test]
    fn test_cleanup_category_all() {
        let all = CleanupCategory::all();
        assert!(all.len() >= 8);
        assert!(all.contains(&CleanupCategory::BotHistory));
        assert!(all.contains(&CleanupCategory::CacheData));
    }

    #[test]
    fn test_cleanup_category_default_enabled() {
        let defaults = CleanupCategory::default_enabled();
        assert!(defaults.contains(&CleanupCategory::BotHistory));
        assert!(defaults.contains(&CleanupCategory::TempFiles));
        assert!(!defaults.contains(&CleanupCategory::AnalyticsRaw));
    }

    #[test]
    fn test_cleanup_config_default() {
        let config = CleanupConfig::default();
        assert_eq!(config.retention_days, DEFAULT_RETENTION_DAYS);
        assert!(!config.auto_cleanup_enabled);
        assert_eq!(config.storage_alert_threshold_percent, 80);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 bytes");
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.00 TB");
    }

    #[test]
    fn test_cleanup_error_display() {
        let err = CleanupError::DatabaseConnection;
        assert_eq!(err.to_string(), "Database connection failed");

        let err = CleanupError::RetentionTooShort(7);
        assert_eq!(err.to_string(), "Retention must be at least 7 days");
    }

    #[test]
    fn test_cleanup_trigger_display() {
        assert_eq!(CleanupTrigger::Manual.to_string(), "Manual");
        assert_eq!(CleanupTrigger::Scheduled.to_string(), "Scheduled");
        assert_eq!(CleanupTrigger::StorageAlert.to_string(), "Storage Alert");
    }

    #[test]
    fn test_category_is_essential() {
        assert!(!CleanupCategory::BotHistory.is_essential());
        assert!(!CleanupCategory::TempFiles.is_essential());
        assert!(CleanupCategory::OldCalendarEvents.is_essential());
        assert!(CleanupCategory::Attachments.is_essential());
    }

    #[test]
    fn test_category_preview_creation() {
        let preview = CategoryPreview {
            category: CleanupCategory::BotHistory,
            display_name: "Bot History".to_string(),
            record_count: 1000,
            estimated_bytes: 1024 * 1024,
            estimated_bytes_formatted: "1.00 MB".to_string(),
            oldest_record: Some(Utc::now()),
            is_essential: false,
        };
        assert_eq!(preview.record_count, 1000);
        assert!(!preview.is_essential);
    }
}

mod lib_part1;
mod lib_part2;
mod lib_part3;
mod lib_part4;
mod lib_part5;
mod lib_part6;
mod lib_part7;
mod lib_part8;
mod lib_part9;

use lib_part7::*;
pub use lib_part8::*;
use lib_part9::*;
