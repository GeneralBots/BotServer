use super::*;

impl VaultConfigProvider for SecretsManagerVaultProvider {
    fn get_secret(&self, path: &str) -> Result<String, String> {
        let v = self.0.get_value_blocking(path, "key", "");
        if v.is_empty() {
            Ok(self.0.get_value_blocking(path, "master_key", ""))
        } else {
            Ok(v)
        }
    }
    fn get_directory_config(&self) -> Result<(String, String, String, String), String> {
        let url = self.0.get_value_blocking("gbo/directory", "url", "");
        let host = self.0.get_value_blocking("gbo/directory", "host", "localhost");
        let port = self.0.get_value_blocking("gbo/directory", "port", "9000");
        let project_id = self.0.get_value_blocking("gbo/directory", "project_id", "");
        Ok((url, host, port, project_id))
    }
    fn get_directory_config_sync(&self) -> (String, String, String, String) {
        self.get_directory_config().unwrap_or_default()
    }
    fn get_vectordb_config_sync(&self) -> (String, Option<String>) { (String::new(), None) }
    fn get_llm_config(&self) -> (String, String, Option<String>, Option<String>, String) {
        (String::new(), String::new(), None, None, String::new())
    }
    fn get_cache_config(&self) -> Result<(String, u16, Option<String>), String> {
        let host = self.0.get_value_blocking("gbo/cache", "host", "localhost");
        let port: u16 = self.0.get_value_blocking("gbo/cache", "port", "6379").parse().unwrap_or(6379);
        let password = self.0.get_value_blocking("gbo/cache", "password", "");
        let pw = if password.is_empty() { None } else { Some(password) };
        Ok((host, port, pw))
    }
    fn get_database_config_sync(&self) -> Result<(String, u16, String, String, String), String> {
        let host = self.0.get_value_blocking("gbo/tables", "host", "localhost");
        let port: u16 = self.0.get_value_blocking("gbo/tables", "port", "5432").parse().unwrap_or(5432);
        let db = self.0.get_value_blocking("gbo/tables", "database", "botserver");
        let user = self.0.get_value_blocking("gbo/tables", "username", "gbuser");
        let pass = self.0.get_value_blocking("gbo/tables", "password", "");
        Ok((host, port, db, user, pass))
    }
    fn get_drive_config(&self) -> Result<(String, String, String), String> {
        let host = self.0.get_value_blocking("gbo/drive", "host", "");
        let access = self.0.get_value_blocking("gbo/drive", "accesskey", "");
        let secret = self.0.get_value_blocking("gbo/drive", "secret", "");
        Ok((host, access, secret))
    }
    fn get_jwt_secret(&self) -> Result<String, String> {
        Ok(self.0.get_value_blocking("gbo/jwt", "secret", ""))
    }
}
pub async fn init_secrets_manager() -> anyhow::Result<()> {
    let manager = SecretsManager::from_env()?;
    let provider = SecretsManagerVaultProvider(manager);
    botsecurity_core::set_vault_provider(Box::new(provider));
    botsecurity_crypto::encryption::reload_master_key_from_vault();
    Ok(())
}
pub fn get_database_url_sync() -> anyhow::Result<String> {
    // Try Vault first via SecretsManager
    if let Ok(manager) = botcoresecrets::SecretsManager::get_clone() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(manager.get_database_url()),
                Err(e) => Err(anyhow::anyhow!("Failed to create runtime: {}", e)),
            };
            let _ = tx.send(result);
        });
        if let Ok(url) = rx.recv().map_err(|e| anyhow::anyhow!("Channel error: {}", e))? {
            std::env::set_var("DATABASE_URL", &url);
            return Ok(url);
        }
    }

    // Fallback to environment variable
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("PG_URL"))
        .map_err(|_| anyhow::anyhow!("DATABASE_URL not set"))
}
pub fn get_work_path() -> String {
    get_work_path_default()
}
/// Resolve and read a bot file across all supported Drive layouts.
///
/// A bot's materialized files can live under the work path in three layouts:
/// 1. Bot-named org bucket (legacy primary): `{bot}.gborg/{bot}.gbai/{file_sub}`
/// 2. Standalone bucket:                     `{bot}.gbai/{file_sub}`
/// 3. Org workspace (SaaS):                  `{org}.gborg/{branch}.gbai/{file_sub}`
///
/// `file_sub` is the path **inside** the `.gbai` bucket, e.g.
/// `{bot}.gbdialog/start.ast`. Layout 3 is discovered by scanning
/// `{work_path}/*.gborg/*.gbai/` because callers (e.g. WebSocket handlers)
/// usually only know the bot name — not the org or branch the bot was synced
/// under. Returns the first non-empty hit.
pub async fn read_bot_file_across_layouts(
    work_path: &str,
    bot_name: &str,
    file_sub: &str,
) -> Option<String> {
    // Safety: reject traversal in the bucket-relative portion.
    let sub = std::path::Path::new(file_sub);
    if sub.is_absolute()
        || file_sub.contains("..")
        || bot_name.contains("..")
        || bot_name.contains('/')
    {
        warn!("read_bot_file_across_layouts: unsafe path rejected: bot={bot_name} sub={file_sub}");
        return None;
    }

    // Layouts 1 and 2 — deterministic, no scan needed.
    for rel in [
        format!("{work_path}/{bot_name}.gborg/{bot_name}.gbai/{file_sub}"),
        format!("{work_path}/{bot_name}.gbai/{file_sub}"),
    ] {
        if let Ok(content) = tokio::fs::read_to_string(&rel).await {
            if !content.is_empty() {
                debug!("read_bot_file_across_layouts: found {rel}");
                return Some(content);
            }
        }
    }

    // Layout 3 — scan org workspaces: {work_path}/{org}.gborg/{branch}.gbai/{file_sub}
    let mut dirs = match tokio::fs::read_dir(work_path).await {
        Ok(d) => d,
        Err(e) => {
            warn!("read_bot_file_across_layouts: cannot list {work_path}: {e}");
            return None;
        }
    };
    while let Ok(Some(entry)) = dirs.next_entry().await {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".gborg") {
            continue;
        }
        let org_dir = entry.path();
        let mut branches = match tokio::fs::read_dir(&org_dir).await {
            Ok(b) => b,
            Err(_) => continue,
        };
        while let Ok(Some(branch)) = branches.next_entry().await {
            let bname = branch.file_name();
            let bname = bname.to_string_lossy();
            if !bname.ends_with(".gbai") {
                continue;
            }
            let full = branch.path().join(sub);
            if let Ok(content) = tokio::fs::read_to_string(&full).await {
                if !content.is_empty() {
                    debug!("read_bot_file_across_layouts: found {}", full.display());
                    return Some(content);
                }
            }
        }
    }
    None
}
/// Returns the current org ID for bot file path isolation.
///
/// Currently always returns Uuid::nil() until real multi-tenant auth
/// is implemented. Once UserSession.branch_id is properly populated
/// from JWT tokens, this function will return the real org UUID.
///
/// When a session context is available, prefer passing the session's
/// branch_id directly to build_bot_path() / get_org_work_path().
pub fn current_org_id() -> uuid::Uuid {
    uuid::Uuid::nil()
}
/// Build a relative bot path with org isolation (.gborg wrapping).
/// Returns: "{org_id}.gborg/{bot_bucket}.gbai/{sub_path}"
///
/// This mirrors the MinIO Drive bucket structure where each org's bots
/// are stored under `{org_id}.gborg/` and each bot under `{bot_name}.gbai/`.
///
/// When `org_id` is not available, use `current_org_id()` which returns
/// the nil UUID (current default for single-tenant mode).
pub fn build_bot_path(org_id: impl std::fmt::Display, bot_bucket: &str, sub_path: &str) -> String {
    format!("{org_id}.gborg/{bot_bucket}.gbai/{sub_path}")
}
/// Build an absolute bot path with org isolation.
/// Returns: "{work_root}/{org_id}.gborg/{bot_bucket}.gbai/{sub_path}"
///
/// Use this for local filesystem paths to bot files (compiled .ast,
/// templates, etc.). The .gborg layer ensures org-level isolation.
pub fn build_absolute_bot_path(
    work_root: &str,
    org_id: impl std::fmt::Display,
    bot_bucket: &str,
    sub_path: &str,
) -> String {
    format!("{work_root}/{org_id}.gborg/{bot_bucket}.gbai/{sub_path}")
}
/// Get work path with org isolation suffix.
/// Returns: "{work_path}/{org_id}.gborg/"
pub fn get_org_work_path(org_id: impl std::fmt::Display) -> String {
    format!("{}/{org_id}.gborg/", get_work_path())
}
/// Returns the work directory path.
/// In production (system container): /opt/gbo/work
/// In development (with botserver-stack directory): ./botserver-stack/data/system/work
pub(super) fn get_work_path_default() -> String {
    if let Ok(path) = std::env::var("GBO_WORK_PATH") {
        if !path.is_empty() {
            return path;
        }
    }

    // Production check: binary running from /opt/gbo/bin with /opt/gbo/work existing
    let prod_exe = std::env::current_exe()
        .ok()
        .is_some_and(|p| p.starts_with("/opt/gbo/bin"));
    let prod_work = std::path::Path::new("/opt/gbo/work").exists();
    if prod_exe && prod_work {
        return "/opt/gbo/work".to_string();
    }

    // Dev fallback: resolve to an absolute path to avoid CWD-dependent breakage.
    // The path is resolved against the current directory at startup and stored
    // once, so the runtime no longer depends on the process working directory.
    let dev_path = std::path::Path::new("./botserver-stack/data/system/work");
    if dev_path.exists() {
        std::fs::canonicalize(dev_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| {
                std::env::current_dir()
                    .map(|cwd| {
                        format!(
                            "{}/botserver-stack/data/system/work",
                            cwd.to_string_lossy()
                        )
                    })
                    .unwrap_or_else(|_| "./botserver-stack/data/system/work".to_string())
            })
    } else {
        std::env::current_dir()
            .map(|cwd| {
                format!(
                    "{}/botserver-stack/data/system/work",
                    cwd.to_string_lossy()
                )
            })
            .unwrap_or_else(|_| "./botserver-stack/data/system/work".to_string())
    }
}
/// Returns the stack base path.
/// In production (system container with .env): /opt/gbo
/// In development: ./botserver-stack
/// The `--stack-path` CLI flag sets `BOTSERVER_STACK_PATH`/`GBO_STACK_PATH`.
pub fn get_stack_path() -> String {
    if let Ok(path) = std::env::var("BOTSERVER_STACK_PATH") {
        if !path.trim().is_empty() {
            return path;
        }
    }
    if let Ok(path) = std::env::var("GBO_STACK_PATH") {
        if !path.trim().is_empty() {
            return path;
        }
    }
    // Production deployment: the stack data lives at `/opt/gbo` and the
    // binary at `/opt/gbo/bin/botserver`. This must be checked before the
    // dev-relative `./botserver-stack` probe below: in prod the process CWD is
    // normally `/opt/gbo/bin`, where a stray `botserver-stack` directory can
    // exist and would otherwise shadow the real stack root.
    if std::path::Path::new("/opt/gbo/bin/botserver").exists()
        || std::path::Path::new("/opt/gbo/bin/.env").exists()
    {
        return "/opt/gbo".to_string();
    }
    // Development checkout: the stack data dir sits next to the repo root.
    // Absent an explicit marker, assume the relative layout for local runs.
    "./botserver-stack".to_string()
}
#[cfg(feature = "drive")]
pub async fn create_s3_operator(
    _config: &DriveConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("S3 operator is not available in botcore — use botdrive instead".into())
}
