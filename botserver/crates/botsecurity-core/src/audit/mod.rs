pub(crate) use anyhow::Result;
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::info;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditEventCategory {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    Administration,
    Security,
    System,
    Compliance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditEventType {
    LoginSuccess,
    LoginFailure,
    Logout,
    MfaEnabled,
    MfaDisabled,
    MfaChallenge,
    PasswordChange,
    PasswordReset,
    SessionCreate,
    SessionRevoke,
    SessionExpire,

    PermissionGranted,
    PermissionDenied,
    RoleAssigned,
    RoleRemoved,
    GroupJoined,
    GroupLeft,

    DataRead,
    DataCreate,
    DataUpdate,
    DataDelete,
    DataShare,
    DataUnshare,
    DataDownload,
    DataExport,

    UserCreate,
    UserDelete,
    UserModify,
    UserDisable,
    UserEnable,
    ConfigChange,
    SettingChange,
    BotCreate,
    BotDelete,
    BotModify,

    ThreatDetected,
    PolicyViolation,
    RateLimitExceeded,
    SuspiciousActivity,
    BruteForceAttempt,
    InjectionAttempt,
    UnauthorizedAccess,

    ServiceStart,
    ServiceStop,
    BackupCreate,
    BackupRestore,
    MaintenanceStart,
    MaintenanceEnd,

    ConsentGiven,
    ConsentWithdrawn,
    DataExportRequest,
    DataDeletionRequest,
    PrivacyPolicyAccepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOutcome {
    Success,
    Failure,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
    pub actor_type: ActorType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorType {
    User,
    Service,
    System,
    Bot,
    Anonymous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResource {
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    pub parent_resource: Option<Box<AuditResource>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub category: AuditEventCategory,
    pub severity: AuditSeverity,
    pub outcome: AuditOutcome,
    pub actor: AuditActor,
    pub resource: Option<AuditResource>,
    pub action: String,
    pub description: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub request_id: Option<String>,
    pub organization_id: Option<Uuid>,
    pub previous_hash: Option<String>,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub async_logging: bool,
    pub buffer_size: usize,
    pub flush_interval_seconds: u64,
    pub retention_days: u32,
    pub min_severity: AuditSeverity,
    pub categories_enabled: Vec<AuditEventCategory>,
    pub tamper_evident: bool,
    pub compress_old_logs: bool,
    pub encrypt_sensitive_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub event_types: Option<Vec<AuditEventType>>,
    pub categories: Option<Vec<AuditEventCategory>>,
    pub severities: Option<Vec<AuditSeverity>>,
    pub outcomes: Option<Vec<AuditOutcome>>,
    pub actor_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub organization_id: Option<Uuid>,
    pub request_id: Option<String>,
    pub search_text: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryResult {
    pub events: Vec<AuditEvent>,
    pub total_count: usize,
    pub has_more: bool,
}

pub trait AuditStore: Send + Sync {
    fn store(&self, event: AuditEvent) -> impl std::future::Future<Output = Result<()>> + Send;
    fn store_batch(&self, events: Vec<AuditEvent>) -> impl std::future::Future<Output = Result<()>> + Send;
    fn query(&self, query: AuditQuery) -> impl std::future::Future<Output = Result<AuditQueryResult>> + Send;
    fn get_by_id(&self, id: Uuid) -> impl std::future::Future<Output = Result<Option<AuditEvent>>> + Send;
    fn get_chain(&self, start_id: Uuid, count: usize) -> impl std::future::Future<Output = Result<Vec<AuditEvent>>> + Send;
    fn verify_chain(&self, start_id: Uuid, end_id: Uuid) -> impl std::future::Future<Output = Result<bool>> + Send;
    fn cleanup_old_events(&self, before: DateTime<Utc>) -> impl std::future::Future<Output = Result<usize>> + Send;
}

#[derive(Debug, Clone)]
pub struct InMemoryAuditStore {
    events: Arc<RwLock<Vec<AuditEvent>>>,
    max_events: usize,
}

pub struct AuditLogger<S: AuditStore> {
    config: AuditConfig,
    store: S,
    buffer: Arc<RwLock<Vec<AuditEvent>>>,
    last_hash: Arc<RwLock<Option<String>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let actor = AuditActor::user(Uuid::new_v4());
        let event = AuditEvent::new(AuditEventType::LoginSuccess, actor);

        assert_eq!(event.event_type, AuditEventType::LoginSuccess);
        assert_eq!(event.category, AuditEventCategory::Authentication);
        assert!(event.verify_hash());
    }

    #[test]
    fn test_audit_event_hash_verification() {
        let actor = AuditActor::user(Uuid::new_v4());
        let event = AuditEvent::new(AuditEventType::DataCreate, actor)
            .with_description("Created new document");

        assert!(event.verify_hash());
    }

    #[test]
    fn test_audit_severity_levels() {
        assert_eq!(
            AuditEventType::LoginSuccess.severity(),
            AuditSeverity::Info
        );
        assert_eq!(
            AuditEventType::LoginFailure.severity(),
            AuditSeverity::Warning
        );
        assert_eq!(
            AuditEventType::ThreatDetected.severity(),
            AuditSeverity::Critical
        );
    }

    #[test]
    fn test_audit_actor_builders() {
        let user_actor = AuditActor::user(Uuid::new_v4())
            .with_username("testuser".into())
            .with_ip("192.168.1.1".into());

        assert!(user_actor.user_id.is_some());
        assert_eq!(user_actor.username, Some("testuser".into()));
        assert_eq!(user_actor.actor_type, ActorType::User);
    }

    #[test]
    fn test_audit_resource_builder() {
        let resource = AuditResource::new("file")
            .with_id("123")
            .with_name("document.pdf");

        assert_eq!(resource.resource_type, "file");
        assert_eq!(resource.resource_id, Some("123".into()));
        assert_eq!(resource.resource_name, Some("document.pdf".into()));
    }

    #[tokio::test]
    async fn test_in_memory_store() {
        let store = InMemoryAuditStore::new(1000);
        let actor = AuditActor::user(Uuid::new_v4());
        let event = AuditEvent::new(AuditEventType::LoginSuccess, actor);
        let event_id = event.id;

        store.store(event).await.expect("Store failed");

        let retrieved = store.get_by_id(event_id).await.expect("Get failed");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|e| e.id), Some(event_id));
    }

    #[tokio::test]
    async fn test_audit_query() {
        let store = InMemoryAuditStore::new(1000);

        for _ in 0..5 {
            let actor = AuditActor::user(Uuid::new_v4());
            let event = AuditEvent::new(AuditEventType::LoginSuccess, actor);
            store.store(event).await.expect("Store failed");
        }

        let query = AuditQuery::new()
            .with_event_types(vec![AuditEventType::LoginSuccess])
            .with_limit(10);

        let result = store.query(query).await.expect("Query failed");
        assert_eq!(result.events.len(), 5);
        assert_eq!(result.total_count, 5);
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let logger = create_audit_logger();
        let actor = AuditActor::user(Uuid::new_v4());

        logger
            .log_auth_success(actor.clone(), "password")
            .await
            .expect("Log failed");

        logger
            .log_auth_failure(actor, "invalid_password")
            .await
            .expect("Log failed");

        logger.flush().await.expect("Flush failed");
    }

    #[test]
    fn test_event_category_mapping() {
        assert_eq!(
            AuditEventType::LoginSuccess.category(),
            AuditEventCategory::Authentication
        );
        assert_eq!(
            AuditEventType::PermissionDenied.category(),
            AuditEventCategory::Authorization
        );
        assert_eq!(
            AuditEventType::DataRead.category(),
            AuditEventCategory::DataAccess
        );
        assert_eq!(
            AuditEventType::UserCreate.category(),
            AuditEventCategory::Administration
        );
        assert_eq!(
            AuditEventType::ThreatDetected.category(),
            AuditEventCategory::Security
        );
    }
}

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
mod part11;

pub use part11::*;
