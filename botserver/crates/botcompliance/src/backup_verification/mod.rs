pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
    Snapshot,
    LogBackup,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackupStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Verified,
    VerificationFailed,
    Expired,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageLocation {
    Local,
    S3,
    Azure,
    Gcs,
    Sftp,
    Nfs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    ChecksumValidation,
    RestoreTest,
    PartialRestore,
    MetadataCheck,
    SampleDataValidation,
    FullIntegrityCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: Uuid,
    pub backup_type: BackupType,
    pub source: String,
    pub storage_location: StorageLocation,
    pub storage_path: String,
    pub size_bytes: u64,
    pub compressed_size_bytes: Option<u64>,
    pub checksum: String,
    pub checksum_algorithm: String,
    pub encryption_key_id: Option<String>,
    pub status: BackupStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub retention_days: u32,
    pub expires_at: DateTime<Utc>,
    pub parent_backup_id: Option<Uuid>,
    pub metadata: BackupMetadata,
    pub verification_history: Vec<VerificationResult>,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub database_version: Option<String>,
    pub application_version: String,
    pub schema_version: Option<String>,
    pub table_count: Option<u32>,
    pub row_count: Option<u64>,
    pub files_count: Option<u32>,
    pub organization_ids: Vec<Uuid>,
    pub custom_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub id: Uuid,
    pub backup_id: Uuid,
    pub method: VerificationMethod,
    pub status: VerificationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub checks_performed: Vec<VerificationCheck>,
    pub errors: Vec<VerificationError>,
    pub warnings: Vec<String>,
    pub verified_by: Option<Uuid>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Pending,
    Running,
    Passed,
    Failed,
    PartialSuccess,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub name: String,
    pub description: String,
    pub passed: bool,
    pub expected_value: Option<String>,
    pub actual_value: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationError {
    pub code: String,
    pub message: String,
    pub severity: ErrorSeverity,
    pub recoverable: bool,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub source_pattern: String,
    pub backup_type: BackupType,
    pub schedule: BackupSchedule,
    pub retention: RetentionPolicy,
    pub storage: StorageConfiguration,
    pub verification: VerificationPolicy,
    pub notifications: NotificationSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub cron_expression: String,
    pub timezone: String,
    pub full_backup_day: Option<u8>,
    pub incremental_frequency_hours: Option<u32>,
    pub maintenance_window_start: Option<String>,
    pub maintenance_window_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub daily_retention_days: u32,
    pub weekly_retention_weeks: u32,
    pub monthly_retention_months: u32,
    pub yearly_retention_years: u32,
    pub minimum_backups: u32,
    pub maximum_backups: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfiguration {
    pub primary_location: StorageLocation,
    pub primary_path: String,
    pub secondary_location: Option<StorageLocation>,
    pub secondary_path: Option<String>,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
    pub compression_level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPolicy {
    pub auto_verify: bool,
    pub verification_methods: Vec<VerificationMethod>,
    pub verify_after_backup: bool,
    pub periodic_verification_days: u32,
    pub restore_test_frequency_days: Option<u32>,
    pub sample_data_percentage: Option<u8>,
    pub fail_on_verification_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub notify_on_success: bool,
    pub notify_on_failure: bool,
    pub notify_on_verification_failure: bool,
    pub notify_on_expiration_warning: bool,
    pub expiration_warning_days: u32,
    pub channels: Vec<String>,
    pub recipients: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreTestResult {
    pub id: Uuid,
    pub backup_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: RestoreTestStatus,
    pub restore_target: String,
    pub data_validated: bool,
    pub tables_restored: u32,
    pub rows_validated: u64,
    pub integrity_checks: Vec<IntegrityCheck>,
    pub performance_metrics: RestorePerformanceMetrics,
    pub errors: Vec<VerificationError>,
    pub cleanup_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestoreTestStatus {
    Pending,
    Restoring,
    Validating,
    CleaningUp,
    Passed,
    Failed,
    PartialSuccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheck {
    pub table_name: String,
    pub expected_rows: u64,
    pub actual_rows: u64,
    pub checksum_match: bool,
    pub foreign_keys_valid: bool,
    pub indexes_valid: bool,
    pub constraints_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorePerformanceMetrics {
    pub total_duration_seconds: u64,
    pub restore_speed_mbps: f32,
    pub validation_duration_seconds: u64,
    pub peak_memory_usage_mb: u64,
    pub disk_io_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupHealthReport {
    pub generated_at: DateTime<Utc>,
    pub report_period_days: u32,
    pub summary: BackupHealthSummary,
    pub backup_status: Vec<BackupStatusEntry>,
    pub verification_status: Vec<VerificationStatusEntry>,
    pub storage_usage: StorageUsageReport,
    pub compliance: ComplianceStatus,
    pub recommendations: Vec<HealthRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupHealthSummary {
    pub total_backups: u32,
    pub successful_backups: u32,
    pub failed_backups: u32,
    pub verified_backups: u32,
    pub verification_failures: u32,
    pub last_successful_backup: Option<DateTime<Utc>>,
    pub last_successful_verification: Option<DateTime<Utc>>,
    pub backup_success_rate: f32,
    pub verification_success_rate: f32,
    pub average_backup_duration_seconds: u64,
    pub rpo_compliance: bool,
    pub rto_estimated_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatusEntry {
    pub backup_id: Uuid,
    pub backup_type: BackupType,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub status: BackupStatus,
    pub verified: bool,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStatusEntry {
    pub verification_id: Uuid,
    pub backup_id: Uuid,
    pub method: VerificationMethod,
    pub status: VerificationStatus,
    pub performed_at: DateTime<Utc>,
    pub issues_found: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsageReport {
    pub total_size_bytes: u64,
    pub primary_storage_bytes: u64,
    pub secondary_storage_bytes: u64,
    pub compression_ratio: f32,
    pub deduplication_ratio: f32,
    pub storage_growth_rate_percent: f32,
    pub estimated_monthly_cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub rpo_target_hours: u32,
    pub rpo_actual_hours: u32,
    pub rpo_compliant: bool,
    pub retention_compliant: bool,
    pub encryption_compliant: bool,
    pub geographic_redundancy_compliant: bool,
    pub verification_schedule_compliant: bool,
    pub last_audit_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthRecommendation {
    pub priority: u32,
    pub category: String,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub impact: String,
}

#[derive(Debug, Clone)]
pub enum BackupError {
    NotFound(String),
    VerificationFailed(String),
    StorageError(String),
    EncryptionError(String),
    PolicyViolation(String),
    RestoreFailed(String),
    InvalidConfiguration(String),
}

pub struct BackupVerificationService {
    backups: Arc<RwLock<HashMap<Uuid, BackupRecord>>>,
    policies: Arc<RwLock<HashMap<Uuid, BackupPolicy>>>,
    verifications: Arc<RwLock<Vec<VerificationResult>>>,
    restore_tests: Arc<RwLock<Vec<RestoreTestResult>>>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
