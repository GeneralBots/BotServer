use super::*;

/// Everything one drive monitor is created with.
pub struct DriveMonitorParams {
    pub state: Arc<AppState>,
    pub bucket_name: String,
    pub bot_id: uuid::Uuid,
    pub bot_name: String,
    pub branch_slug: String,
    pub s3_prefix: Option<String>,
    pub tenant_slug: String,
    pub org_slug: String,
}

impl DriveMonitor {
    pub fn new_with_params(params: DriveMonitorParams) -> Self {
        let DriveMonitorParams {
            state,
            bucket_name,
            bot_id,
            bot_name,
            branch_slug,
            s3_prefix,
            tenant_slug,
            org_slug,
        } = params;
        let work_root = PathBuf::from(botcore::shared::utils::get_work_path());
        #[cfg(any(feature = "research", feature = "llm"))]
        let kb_manager = Arc::new(KnowledgeBaseManager::with_bot_config(
            work_root.clone(),
            state.conn.clone(),
            bot_id,
        ));

        let file_repo = Arc::new(DriveFileRepository::new(state.conn.clone()));

        Self {
            state,
            bucket_name,
            bot_id,
            bot_name,
            branch_slug,
            s3_prefix,
            tenant_slug,
            org_slug,
            #[cfg(any(feature = "research", feature = "llm"))]
            kb_manager,
            work_root,
            is_processing: Arc::new(AtomicBool::new(false)),
            scanning: Arc::new(AtomicBool::new(false)),
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            #[cfg(any(feature = "research", feature = "llm"))]
            files_being_indexed: Arc::new(TokioRwLock::new(HashSet::new())),
            #[cfg(any(feature = "research", feature = "llm"))]
            pending_kb_index: Arc::new(TokioRwLock::new(HashSet::new())),
            #[cfg(any(feature = "research", feature = "llm"))]
            kb_indexed_folders: Arc::new(TokioRwLock::new(HashSet::new())),
            #[cfg(not(any(feature = "research", feature = "llm")))]
            _pending_kb_index: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
            file_repo,
        }
    }

    pub(super) fn strip_prefix<'a>(&self, key: &'a str) -> &'a str {
        if let Some(prefix) = &self.s3_prefix {
            key.strip_prefix(prefix.as_str()).unwrap_or(key)
        } else {
            key
        }
    }

    pub(super) fn bot_work_dir(&self, subdir: &str) -> PathBuf {
        if &self.tenant_slug == "default" && &self.org_slug == "default" {
            // Flat bot (no tenant/org isolation) — store directly under work_root
            // Wrapped in .gborg to mirror Drive bucket structure (Issue #712)
            self.work_root
                .join(format!("{org_id}.gborg/{branch_slug}.gbai/{bot_name}.{subdir}", org_id = current_org_id(), branch_slug = self.branch_slug, bot_name = self.bot_name))
        } else {
            // Multi-tenant bot — store under tenant/org prefix with .gborg wrapping
            self.work_root
                .join(&self.tenant_slug)
                .join(&self.org_slug)
                .join(format!("{org_id}.gborg/{branch_slug}.gbai/{bot_name}.{subdir}", org_id = current_org_id(), branch_slug = self.branch_slug, bot_name = self.bot_name))
        }
    }
}
