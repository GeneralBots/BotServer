pub mod antivirus;
pub mod audit;
pub mod command_guard;
pub mod cors;
pub mod dlp;
pub mod error_sanitizer;
pub mod file_validation;
pub mod headers;
pub mod log_sanitizer;
pub mod panic_handler;
pub mod path_guard;
pub mod prompt_security;
pub mod request_id;
pub mod safe_unwrap;
pub mod scope;
pub mod security_monitoring;
pub mod sql_guard;
pub mod tenant;
pub mod validation;
pub mod webhook;

use uuid::Uuid;

pub trait VaultConfigProvider: Send + Sync {
    fn get_secret(&self, path: &str) -> Result<String, String>;
    fn get_directory_config(&self) -> Result<(String, String, String, String), String>;
    fn get_directory_config_sync(&self) -> (String, String, String, String);
    fn get_vectordb_config_sync(&self) -> (String, Option<String>);
    fn get_llm_config(&self) -> (String, String, Option<String>, Option<String>, String);
    fn get_cache_config(&self) -> Result<(String, u16, Option<String>), String>;
    fn get_database_config_sync(&self) -> Result<(String, u16, String, String, String), String>;
    fn get_drive_config(&self) -> Result<(String, String, String), String>;
    fn get_jwt_secret(&self) -> Result<String, String>;
}

pub trait SessionCacheProvider: Send + Sync {
    fn try_read_session_cache<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&std::sync::RwLockReadGuard<'_, dyn std::any::Any>) -> R;
}

pub type SessionCacheLookupFn = Box<dyn Fn(&str) -> Option<SessionCacheEntry> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct SessionCacheEntry {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
    /// Org scope carried from the login JWT (cloud SSO) so org-scoped suite
    /// APIs (vibe projects, metering, domains) resolve the user's real
    /// organization instead of the nil scope (fix #1268).
    pub organization_id: Option<Uuid>,
}

use std::sync::OnceLock;
static VAULT_PROVIDER: OnceLock<Box<dyn VaultConfigProvider>> = OnceLock::new();
static SESSION_CACHE_LOOKUP: OnceLock<SessionCacheLookupFn> = OnceLock::new();

pub fn set_vault_provider(provider: Box<dyn VaultConfigProvider>) {
    let _ = VAULT_PROVIDER.set(provider);
}

pub fn get_vault_provider() -> Option<&'static dyn VaultConfigProvider> {
    VAULT_PROVIDER.get().map(|b| b.as_ref())
}

pub fn set_session_cache_lookup(lookup: SessionCacheLookupFn) {
    let _ = SESSION_CACHE_LOOKUP.set(lookup);
}

pub fn lookup_session_cache(session_id: &str) -> Option<SessionCacheEntry> {
    SESSION_CACHE_LOOKUP.get().and_then(|f| f(session_id))
}

mod re_cache;
