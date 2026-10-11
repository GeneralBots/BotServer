pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use crate::shared::utils::DbPool;
pub(crate) use botcoresecrets::manager::SecretsManager;
pub(crate) use botsecurity_crypto::encryption::{decrypt_field, derive_scope_key, load_master_encryption_key};
pub(crate) use botsecurity_crypto::secrets::is_sensitive_key;
pub(crate) use diesel::prelude::*;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub drive: DriveConfig,
    pub email: EmailConfig,
    pub site_path: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriveConfig {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub server: String,
    /// Which object-storage backend the endpoint belongs to: `minio`, `b2`,
    /// `wasabi` or `r2` (#1468). Empty means `minio`.
    pub backend: String,
    /// Secondary endpoint used for read failover. Empty disables failover.
    pub fallback_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub from_address: String,
}

#[derive(Debug, Clone, QueryableByName)]
struct ConfigRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    config_value: String,
}

#[derive(Debug, Clone, QueryableByName)]
struct ConfigRowPair {
    #[diesel(sql_type = diesel::sql_types::Text)]
    config_key: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    config_value: String,
}

#[derive(Debug, Clone, QueryableByName)]
struct BotIdentityRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    org_id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    branch_id: uuid::Uuid,
}

/// Configuration manager for runtime config updates.
/// Sensitive keys (tokens, passwords, API keys) → Vault at `gbo/bot/{org_id}/{branch_id}/{bot_id}`.
/// Non-sensitive keys → `bot_configuration` database table.
pub struct ConfigManager {
    pool: Arc<DbPool>,
}

    /// Same as get_config but returns error instead of default on miss.

pub use AppConfig as Config;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

use part4::*;
