pub(crate) use botbasic_core::utils::BotDriveLocation;
pub(crate) use diesel::r2d2::{ConnectionManager, Pool};
pub(crate) use diesel::PgConnection;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::{Arc, RwLock};
pub(crate) use tokio::sync::broadcast;
pub(crate) use uuid::Uuid;

pub type DbPool = Arc<Pool<ConnectionManager<PgConnection>>>;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, BoxError>> + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub context_data: serde_json::Value,
    pub current_tool: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskProgressEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub task_id: String,
    pub step: String,
    pub message: String,
    pub progress: u8,
    pub total_steps: u8,
    pub current_step: u8,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<AgentActivity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentActivity {
    pub phase: String,
    pub items_processed: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items_total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_per_min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_processed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_created: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tables_created: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_lines: Option<Vec<String>>,
}

pub trait AutoTaskState: Send + Sync {
    fn db_pool(&self) -> &DbPool;
    fn bucket_name(&self) -> &str;
    /// Drive object-store facade used by the BASIC-only pipeline (#754).
    fn file_ops(&self) -> Option<&dyn DriveOps>;
    fn broadcast_task_progress(&self, event: TaskProgressEvent);
    fn emit_activity(
        &self,
        task_id: &str,
        step: &str,
        message: &str,
        current: u8,
        total: u8,
        activity: AgentActivity,
    );
    fn emit_task_started(&self, task_id: &str, message: &str, total_steps: u8);
    fn emit_task_error(&self, task_id: &str, step: &str, error: &str);
    fn task_manifests(&self) -> &Arc<RwLock<HashMap<String, crate::TaskManifest>>>;
    fn task_progress_broadcast(&self) -> Option<&broadcast::Sender<TaskProgressEvent>>;
    /// Git source facade (reform #1501/#1505). `None` keeps the legacy Drive
    /// write path for bots that are not git-owned.
    fn source_ops(&self) -> Option<&dyn BotSourceOps> {
        None
    }

    /// BASIC reference handed to the intent compiler: the closed keyword
    /// catalog plus the syntax rules, rendered as prompt material.
    ///
    /// The catalog lives in the host (botserver's `basic::keywords`
    /// registration) because `botautotask` cannot depend on it — without this
    /// the compiler is asked for a program while knowing no keyword and emits
    /// the `TALK` stub instead. `None` degrades to the stub.
    fn basic_reference(&self) -> Option<String> {
        None
    }
}

/// Resolved bot identity used to build Drive buckets and DriveMonitor keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotInfo {
    pub id: Uuid,
    pub name: String,
    /// Where the bot's files physically live. Resolved from the `bots` and
    /// `organizations` rows instead of formatted from the name: an org-hosted
    /// bot lives in the shared `{slug}.gborg` workspace, and a standalone bot
    /// owns its `{name}.gbai` bucket — both shapes occur in prod.
    pub drive: BotDriveLocation,
}

/// Persisted AutoTask row fragment used by the API pipeline (no full
/// agent-state machinery required to record a created task).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub bot_id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub intent: String,
    pub status: String,
    pub mode: String,
    pub priority: String,
    pub plan_id: Option<String>,
    pub basic_program: Option<String>,
}

pub trait BotDatabaseOps: Send + Sync {
    fn create_table_in_bot_database(
        &self,
        bot_id: Uuid,
        sql: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Resolve a bot by its database identifier (name + Drive layout).
    fn resolve_bot(&self, bot_id: Uuid) -> Result<Option<BotInfo>, BoxError>;

    /// Persist an AutoTask metadata row created by the API pipeline
    /// (status, plan id, generated BASIC program) so it is observable in
    /// `/api/autotask/tasks`.
    fn persist_task(&self, task: &TaskRecord) -> Result<(), BoxError>;
}

pub trait LlmProviderOps: Send + Sync {
    fn generate_stream(
        &self,
        prompt: &str,
        config: &serde_json::Value,
        tx: tokio::sync::mpsc::Sender<String>,
        model: &str,
        key: &str,
        system_prompt: Option<&str>,
    ) -> BoxFuture<()>;
}

/// Hard ceiling on one AutoTask LLM completion. A stalled upstream model (free
/// tiers do stall) must degrade into a reported failure instead of holding the
/// request and its HTTP client open — an intent classification once hung for
/// more than five minutes.
/// Generous, because the compile step asks a reasoning model for a whole BASIC
/// program: 90 s aborted real runs on Telegram with "llm call timed out"
/// (the model was still emitting reasoning tokens). The budget is a safety net
/// against a stalled provider, not a latency target.
pub const LLM_CALL_TIMEOUT_SECS: u64 = 300;

pub trait ConfigOps: Send + Sync {
    fn get_config(
        &self,
        bot_id: &Uuid,
        key: &str,
        default: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    fn set_config(
        &self,
        bot_id: &Uuid,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub trait DriveOps: Send + Sync {
    /// Upload a file to MinIO/Drive. Bucket follows the Drive layout
    /// (`{bot}.gbai`), key is the object path inside it.
    fn put_object(
        &self,
        bucket: &str,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<(), BoxError>;

    /// Read an object back (verification + rollback support).
    fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, BoxError>;
}

/// Reform #1501/#1505 — persistence of AutoTask-generated bot sources into the
/// bot's git repository (ALM). For a git-owned bot the repository is the
/// canonical `.gbdialog`; a Drive-only write is overwritten by the next
/// git-pull monitor tick and leaves no artifact of the task behind.
///
/// Implemented by the host crate; `None` for bots without a git project, in
/// which case the legacy Drive path is used.
/// One source file of a bot's `.gbdialog`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub name: String,
    pub size: u64,
}

/// A bot's source files plus the vibe project that owns the repository, so a
/// caller can open them in the suite editor (project workspace + Source
/// Control) instead of guessing a path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceListing {
    pub project_id: Option<Uuid>,
    pub files: Vec<SourceFile>,
}

pub trait BotSourceOps: Send + Sync {
    /// Write `files` — `(path relative to .gbdialog, content)` — into the bot's
    /// source repository, commit and push them, returning the dialog-root file
    /// names written.
    fn write_sources(
        &self,
        bot_id: Uuid,
        files: &[(String, String)],
        message: &str,
    ) -> Result<Vec<String>, BoxError>;

    /// Merge the `BEGIN TABLE … END TABLE` blocks of `tables_bas` into the bot's
    /// `.gbdialog/tables.bas`, returning the table names appended (empty when
    /// the schema already declared them).
    fn merge_tables(&self, bot_id: Uuid, tables_bas: &str) -> Result<Vec<String>, BoxError>;

    /// Write the bot's `.gbot` configuration — `PROMPT-{CHANNEL}.md`, styles,
    /// `config.csv` — into the repository, commit and push it. The runtime reads
    /// these files from the work layout; without them the bot answers with the
    /// generic fallback prompt and its tools (e.g. `classify_media`) are never
    /// called, so a shipped template must deliver them with its tool.
    fn write_bot_config(
        &self,
        bot_id: Uuid,
        files: &[(String, String)],
        message: &str,
    ) -> Result<Vec<String>, BoxError>;

    /// Read one source file back from the bot's repository, so the editor can
    /// show the current committed content. `None` when the file is absent.
    fn read_source(&self, bot_id: Uuid, name: &str) -> Result<Option<String>, BoxError>;

    /// List the bot's `.gbdialog` sources with the owning project id.
    fn list_sources(&self, bot_id: Uuid) -> Result<SourceListing, BoxError>;
}

pub trait ScriptRunner: Send + Sync {
    fn run_script(
        &self,
        script_name: &str,
        bot_id: Uuid,
        session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(test)]
mod tests {
    use super::{collect_llm_stream, BoxError, BoxFuture, LlmProviderOps};

    /// Emits `chunks` messages before optionally failing — enough chunks to
    /// overflow the bounded channel `collect_llm_stream` builds (cap 100).
    struct ManyChunkLlm {
        chunks: usize,
        fail_after: bool,
    }

    impl LlmProviderOps for ManyChunkLlm {
        fn generate_stream(
            &self,
            _prompt: &str,
            _config: &serde_json::Value,
            tx: tokio::sync::mpsc::Sender<String>,
            _model: &str,
            _key: &str,
            _system_prompt: Option<&str>,
        ) -> BoxFuture<()> {
            let chunks = self.chunks;
            let fail_after = self.fail_after;
            Box::pin(async move {
                for i in 0..chunks {
                    if tx.send(format!("chunk-{i}")).await.is_err() {
                        return Ok(()); // collector dropped
                    }
                }
                if fail_after {
                    return Err(BoxError::from("producer failed after streaming"));
                }
                Ok(())
            })
        }
    }

    #[test]
    fn collects_more_chunks_than_the_channel_holds() {
        // The deadlock regression: a reasoning model writing a whole BASIC
        // program emits hundreds of chunks. A collector that awaits the
        // producer before draining blocks on `send` #101, times out after
        // LLM_CALL_TIMEOUT_SECS, and the AutoTask turn dies with no artifact
        // and no reply (prod, 2026-09-29).
        let adapter = ManyChunkLlm { chunks: 250, fail_after: false };
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let response = rt.block_on(async {
            collect_llm_stream(&adapter, "p", &serde_json::json!({}), "m", "k", None)
                .await
                .expect("collect must succeed under the timeout")
        });
        assert_eq!(response.matches("chunk-").count(), 250);
        assert!(response.ends_with("chunk-249"));
    }

    #[test]
    fn producer_error_fails_the_collect_even_with_partial_output() {
        // A gateway drop mid-stream must surface as an error (the caller
        // reports and can retry), not as the partial text — the compiler's
        // parse fallback would otherwise fabricate a plan from truncated
        // output and claim success.
        let adapter = ManyChunkLlm { chunks: 5, fail_after: true };
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let result = rt.block_on(async {
            collect_llm_stream(&adapter, "p", &serde_json::json!({}), "m", "k", None).await
        });
        let err = result.expect_err("producer error must propagate");
        assert!(err.to_string().contains("producer failed after streaming"));
    }
}

#[cfg(test)]
mod bot_info_drive_tests {
    use super::*;
    use botbasic_core::utils::resolve_bot_drive_location;

    fn info(bot: &str, branch_slug: Option<&str>, org_slug: Option<&str>) -> BotInfo {
        BotInfo {
            id: Uuid::nil(),
            name: bot.to_string(),
            drive: resolve_bot_drive_location(bot, branch_slug, org_slug),
        }
    }

    #[test]
    fn standalone_bot_uses_its_own_gbai_bucket() {
        let i = info("beiner", None, None);
        assert_eq!(i.bucket_name(), "beiner.gbai");
        assert_eq!(i.dialog_folder(), "beiner.gbdialog");
    }

    #[test]
    fn org_hosted_bot_uses_gborg_bucket_and_prefixed_key() {
        // Prod shape: org slug `beiner` owns bucket `beiner.gborg` and the bot
        // lives under the `beiner.gbai/` workspace. Writing to a standalone
        // `beiner.gbai` bucket put the file where nothing reads it.
        let i = info("beiner", Some("beiner"), Some("beiner"));
        assert_eq!(i.bucket_name(), "beiner.gborg");
        assert_eq!(i.dialog_folder(), "beiner.gbai/beiner.gbdialog");
    }

    #[test]
    fn legacy_org_slug_does_not_prefix_the_bucket() {
        // `cristo-org` is the DB alias; the physical bucket is `cristo.gborg`.
        let i = info("cristo", Some("cristo"), Some("cristo-org"));
        assert_eq!(i.bucket_name(), "cristo.gborg");
        assert_eq!(i.dialog_folder(), "cristo.gbai/cristo.gbdialog");
    }

    #[test]
    fn dialog_folder_follows_the_branch_not_the_bot_name() {
        // Prod shape: bot `oppbot` lives in branch `opportunity-oppbot`.
        let i = info("oppbot", Some("opportunity-oppbot"), Some("opportunity-oppbot-org"));
        assert_eq!(i.bucket_name(), "opportunity-oppbot.gborg");
        assert_eq!(
            i.dialog_folder(),
            "opportunity-oppbot.gbai/oppbot.gbdialog"
        );
    }

    #[test]
    fn dialog_key_is_not_placed_under_gbdrive() {
        // `.gbdialog` sits beside `.gbdrive` in the bot tree, never inside it.
        for (branch, slug) in [(None, None), (Some("beiner"), Some("beiner"))] {
            let folder = info("beiner", branch, slug).dialog_folder();
            assert!(
                !folder.contains(".gbdrive"),
                "slug {slug:?} produced a dialog key inside .gbdrive: {folder}"
            );
        }
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part4::*;
pub use part5::*;
