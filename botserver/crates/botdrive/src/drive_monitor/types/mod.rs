pub(crate) use botcore::shared::state::AppState;
pub(crate) use botcore::shared::utils::current_org_id;
pub(crate) use super::utils::is_source_prefix;
pub(crate) use crate::drive_files::DriveFileRepository;
#[cfg(any(feature = "research", feature = "llm"))]
pub(crate) use botcore::kb::KnowledgeBaseManager;
pub(crate) use botsecurity_crypto::secrets::is_sensitive_key;
pub(crate) use chrono::Utc;
pub(crate) use diesel::RunQueryDsl;
pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
pub(crate) use std::sync::Arc;
#[cfg(any(feature = "research", feature = "llm"))]
pub(crate) use std::collections::HashSet;
#[cfg(any(feature = "research", feature = "llm"))]
pub(crate) use tokio::sync::RwLock as TokioRwLock;

#[derive(Debug, Clone, diesel::QueryableByName)]
struct BranchIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: uuid::Uuid,
}

    #[cfg(any(feature = "research", feature = "llm"))]

    #[cfg(any(feature = "research", feature = "llm"))]

#[cfg(any(feature = "research", feature = "llm"))]
struct KbPathParts {
    kb_name: String,
    file_name: String,
    relative_path: String,
}

#[derive(Debug, Clone)]
pub struct DriveMonitor {
    pub state: Arc<AppState>,
    pub bucket_name: String,
    pub bot_id: uuid::Uuid,
    pub bot_name: String,
    pub branch_slug: String,
    pub s3_prefix: Option<String>,
    pub tenant_slug: String,
    pub org_slug: String,
    #[cfg(any(feature = "research", feature = "llm"))]
    pub kb_manager: Arc<KnowledgeBaseManager>,
    pub work_root: PathBuf,
    pub is_processing: Arc<AtomicBool>,
    pub scanning: Arc<AtomicBool>,
    pub consecutive_failures: Arc<AtomicU32>,
    #[cfg(any(feature = "research", feature = "llm"))]
    pub files_being_indexed: Arc<TokioRwLock<HashSet<String>>>,
    #[cfg(any(feature = "research", feature = "llm"))]
    pub pending_kb_index: Arc<TokioRwLock<HashSet<String>>>,
    #[cfg(any(feature = "research", feature = "llm"))]
    pub kb_indexed_folders: Arc<TokioRwLock<HashSet<String>>>,
    #[cfg(not(any(feature = "research", feature = "llm")))]
    pub _pending_kb_index: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    pub file_repo: Arc<DriveFileRepository>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

pub use part1::*;
use part5::*;
use part6::*;
