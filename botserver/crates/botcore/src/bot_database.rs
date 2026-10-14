//! Bot Database Management Module
//!
//! This module handles per-bot database management, including:
//! - Getting bot database names from the bots table
//! - Creating connection pools to bot-specific databases
//! - Ensuring bot databases exist and are properly initialized
//! - Syncing bot databases on server startup

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use log::info;
use std::sync::{Arc, RwLock};


/// Cache for bot database connection pools
pub struct BotDatabaseManager {
    /// Main database pool (for accessing bots table)
    main_pool: DbPool,
    /// Cached connection pools for bot databases
    bot_pools: Arc<RwLock<HashMap<Uuid, DbPool>>>,
    /// Base connection URL (without database name)
    base_url: String,
}

#[derive(QueryableByName, Debug)]
pub struct BotDatabaseInfo {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub database_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub branch_slug: Option<String>,
}

#[derive(QueryableByName)]
struct DbExists {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    exists: bool,
}

impl BotDatabaseManager {
    /// Create a new BotDatabaseManager
    pub fn new(main_pool: DbPool, database_url: &str) -> Self {
        let base_url = Self::extract_base_url(database_url);
        Self {
            main_pool,
            bot_pools: Arc::new(RwLock::new(HashMap::new())),
            base_url,
        }
    }

    /// Extract base URL without database name
    /// Converts "postgres://user:pass@host:port/dbname" to "postgres://user:pass@host:port"
    fn extract_base_url(database_url: &str) -> String {
        if let Some(last_slash_pos) = database_url.rfind('/') {
            // Check if there's a query string
            let db_part = &database_url[last_slash_pos..];
            if let Some(query_pos) = db_part.find('?') {
                // Keep query string, just remove db name
                format!(
                    "{}{}",
                    &database_url[..last_slash_pos],
                    &db_part[query_pos..]
                )
            } else {
                database_url[..last_slash_pos].to_string()
            }
        } else {
            database_url.to_string()
        }
    }

    /// Get the database name for a specific bot
    pub fn get_bot_database_name(
        &self,
        bot_id: Uuid,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.main_pool.get()?;

        let result: Option<BotDatabaseInfo> = sql_query(
            "SELECT b.id, b.name, b.database_name, br.slug AS branch_slug \
             FROM bots b LEFT JOIN branches br ON b.branch_id = br.id \
             WHERE b.id = $1 AND b.is_active = true",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .get_result(&mut conn)
        .optional()?;

        Ok(result.and_then(|info| info.database_name))
    }

    /// Get or create a connection pool to a bot's specific database
    pub fn get_bot_pool(
        &self,
        bot_id: Uuid,
    ) -> Result<DbPool, Box<dyn std::error::Error + Send + Sync>> {
        // Check cache first
        {
            let pools = self.bot_pools.read().map_err(|e| format!("Lock error: {}", e))?;
            if let Some(pool) = pools.get(&bot_id) {
                return Ok(<DbPool as Clone>::clone(pool));
            }
        }

        // Get bot info (including name) from database
        let mut conn = self.main_pool.get()?;
        let bot_info: Option<BotDatabaseInfo> = sql_query(
            "SELECT b.id, b.name, b.database_name, br.slug AS branch_slug \
             FROM bots b LEFT JOIN branches br ON b.branch_id = br.id \
             WHERE b.id = $1 AND b.is_active = true",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .get_result(&mut conn)
        .optional()?;

        let bot_info = bot_info.ok_or_else(|| format!("Bot {} not found or not active", bot_id))?;

        // Ensure bot has a database, create if needed
        let branch_slug = bot_info.branch_slug.as_deref().unwrap_or("default");
        let db_name = if let Some(name) = bot_info.database_name {
            name
        } else {
            // Bot doesn't have a database configured, create it now
            info!("Bot {} ({}) has no database, creating now", bot_info.name, bot_id);
            self.ensure_bot_has_database(bot_id, &bot_info.name, branch_slug)?
        };

        // Create new pool
        let pool = self.create_pool_for_database(&db_name)?;

        // Cache it
        {
            let mut pools = self.bot_pools.write().map_err(|e| format!("Lock error: {}", e))?;
            pools.insert(bot_id, pool.clone());
        }

        Ok(pool)
    }

    /// Create a connection pool for a specific database
    fn create_pool_for_database(
        &self,
        database_name: &str,
    ) -> Result<DbPool, Box<dyn std::error::Error + Send + Sync>> {
        let database_url = format!("{}/{}", self.base_url, database_name);
        let manager = ConnectionManager::<PgConnection>::new(&database_url);

        Pool::builder()
            .max_size(5) // Smaller pool for per-bot databases
            .min_idle(Some(0))
            .connection_timeout(std::time::Duration::from_secs(5))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)))
            .build(manager)
            .map_err(|e| format!("Failed to create pool for database {}: {}", database_name, e).into())
    }

    /// Create a database if it doesn't exist
    pub fn ensure_database_exists(
        &self,
        database_name: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // #1447 S1 — the sanitized name is interpolated into pg_database
        // text and DDL below; the strict ASCII allowlist replaces the
        // unicode-permissive `is_alphanumeric` filter (unicode digits would
        // pass it and still interpolate into SQL).
        let safe_db_name: String = database_name
            .chars()
            .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
            .collect();

        if safe_db_name.is_empty() || safe_db_name.len() > 63 {
            return Err("Invalid database name".into());
        }
        if let Err(bad) = crate::project_db::assert_safe_db_name(&safe_db_name) {
            return Err(bad.into());
        }

        let mut conn = self.main_pool.get()?;

        // Check if database exists
        let check_query = format!(
            "SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = '{}') as exists",
            safe_db_name
        );

        let exists = sql_query(&check_query)
            .get_result::<DbExists>(&mut conn)
            .map(|r| r.exists)
            .unwrap_or(false);

        if exists {
            info!("Database {} already exists", safe_db_name);
            return Ok(false); // Already existed
        }

        // Create database
        let create_query = format!("CREATE DATABASE {}", safe_db_name);
        if let Err(e) = sql_query(&create_query).execute(&mut conn) {
            let err_str = e.to_string();
            if err_str.contains("already exists") {
                info!("Database {} already exists (concurrent creation)", safe_db_name);
                return Ok(false);
            }
            return Err(format!("Failed to create database: {}", e).into());
        }

        info!("Created database: {}", safe_db_name);
        Ok(true) // Newly created
    }

    /// Generate a database name for a bot: bot_{branch_slug}_{bot_name}
    /// #1386 — the `_dev` suffix is reserved for Vibe dev-twin databases:
    /// a bot named `{bot}-dev` (the twin) naturally resolves to
    /// `bot_{branch}_{bot}_dev`, never colliding with the production bot.
    pub fn generate_database_name(branch_slug: &str, bot_name: &str) -> String {
        let clean = |s: &str| -> String {
            s.replace(['-', ' '], "_")
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        };
        format!("bot_{}_{}", clean(branch_slug), clean(bot_name))
    }

    /// #1386 — true when a bot name is reserved for the Vibe dev-twin
    /// convention (`{bot}-dev`). Manually created bots must not take these
    /// names, or the twin's per-environment database isolation breaks.
    pub fn is_reserved_dev_bot_name(bot_name: &str) -> bool {
        bot_name.ends_with("-dev") || bot_name == "dev"
    }

    /// Ensure a bot has a database and update the bots table if needed
    pub fn ensure_bot_has_database(
        &self,
        bot_id: Uuid,
        bot_name: &str,
        branch_slug: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Check if bot already has a database_name
        let existing_db_name = self.get_bot_database_name(bot_id)?;

        let db_name = if let Some(name) = existing_db_name {
            name
        } else {
            // Generate and set database name
            let new_db_name = Self::generate_database_name(branch_slug, bot_name);
            let mut conn = self.main_pool.get()?;

            sql_query("UPDATE bots SET database_name = $1 WHERE id = $2")
                .bind::<diesel::sql_types::Varchar, _>(&new_db_name)
                .bind::<diesel::sql_types::Uuid, _>(bot_id)
                .execute(&mut conn)?;

            info!("Set database_name for bot {} to {}", bot_id, new_db_name);
            new_db_name
        };

        // Ensure the database exists
        self.ensure_database_exists(&db_name)?;

        Ok(db_name)
    }

    /// Get all active bots and their database info
    pub fn get_all_bots(&self) -> Result<Vec<BotDatabaseInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.main_pool.get()?;

        let bots: Vec<BotDatabaseInfo> = sql_query(
            "SELECT b.id, b.name, b.database_name, br.slug AS branch_slug \
             FROM bots b LEFT JOIN branches br ON b.branch_id = br.id \
             WHERE b.is_active = true",
        )
        .get_results(&mut conn)?;

        Ok(bots)
    }

    /// Sync all bot databases - ensures each bot has a database
    /// Call this during server startup
    pub fn sync_all_bot_databases(&self) -> Result<SyncResult, Box<dyn std::error::Error + Send + Sync>> {
        let bots = self.get_all_bots()?;
        let mut result = SyncResult::default();

        for bot in bots {
            let branch_slug = bot.branch_slug.as_deref().unwrap_or("default");
            match self.ensure_bot_has_database(bot.id, &bot.name, branch_slug) {
                Ok(db_name) => {
                    if bot.database_name.is_none() {
                        result.databases_created += 1;
                        info!("Created database for bot {}: {}", bot.name, db_name);
                    } else {
                        result.databases_verified += 1;
                    }
                }
                Err(e) => {
                    log::error!("Failed to ensure database for bot {}: {}", bot.name, e);
                    result.errors.push(format!("Bot {}: {}", bot.name, e));
                }
            }
        }

        info!(
            "Bot database sync complete: {} created, {} verified, {} errors",
            result.databases_created,
            result.databases_verified,
            result.errors.len()
        );

        Ok(result)
    }

    /// Execute a table creation SQL in a bot's database
    pub fn create_table_in_bot_database(
        &self,
        bot_id: Uuid,
        create_sql: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.get_bot_pool(bot_id).map_err(|e| format!("No database pool for bot {bot_id}: {e}"))?;
        let mut conn = pool.get()?;

        sql_query(create_sql).execute(&mut conn)?;

        Ok(())
    }

    /// Clear cached pool for a bot (useful when database is recreated)
    pub fn clear_bot_pool_cache(&self, bot_id: Uuid) {
        if let Ok(mut pools) = self.bot_pools.write() {
            let _: Option<_> = pools.remove(&bot_id);
        }
    }

    /// Clear all cached pools
    pub fn clear_all_pool_caches(&self) {
        if let Ok(mut pools) = self.bot_pools.write() {
            std::collections::HashMap::clear(&mut pools);
        }
    }

}

/// Result of syncing bot databases
#[derive(Default, Debug)]
pub struct SyncResult {
    pub databases_created: usize,
    pub databases_verified: usize,
    pub errors: Vec<String>,
}

/// Helper function to create a bot database manager from AppState
pub fn create_bot_database_manager(pool: DbPool, database_url: &str) -> BotDatabaseManager {
    BotDatabaseManager::new(pool, database_url)
}

impl std::fmt::Debug for BotDatabaseManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotDatabaseManager")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl botlib::traits::BotDatabaseService for BotDatabaseManager {
    fn get_bot_pool(&self, bot_id: uuid::Uuid) -> Option<crate::shared::utils::DbPool> {
        self.get_bot_pool(bot_id).ok()
    }

    fn create_table_in_bot_database(&self, bot_id: uuid::Uuid, sql: &str) -> Result<(), String> {
        BotDatabaseManager::create_table_in_bot_database(self, bot_id, sql)
            .map_err(|e| e.to_string())
    }

    fn sync_all_bot_databases(&self) -> Result<(), String> {
        self.sync_all_bot_databases().map(|_| ()).map_err(|e| e.to_string())
    }
}

use diesel::sql_query;
use diesel::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;
use crate::shared::utils::DbPool;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_base_url() {
        assert_eq!(
            BotDatabaseManager::extract_base_url("postgres://user:pass@localhost:5432/mydb"),
            "postgres://user:pass@localhost:5432"
        );

        assert_eq!(
            BotDatabaseManager::extract_base_url("postgres://user:pass@localhost:5432/mydb?sslmode=require"),
            "postgres://user:pass@localhost:5432?sslmode=require"
        );

        assert_eq!(
            BotDatabaseManager::extract_base_url("postgres://user:pass@localhost/mydb"),
            "postgres://user:pass@localhost"
        );
    }

    #[test]
    fn test_generate_database_name() {
        assert_eq!(
            BotDatabaseManager::generate_database_name("default", "my-bot"),
            "bot_default_my_bot"
        );
        assert_eq!(
            BotDatabaseManager::generate_database_name("branch1", "My Bot 2"),
            "bot_branch1_my_bot_2"
        );

        assert_eq!(
            BotDatabaseManager::generate_database_name("prod", "test@bot!"),
            "bot_prod_testbot"
        );
    }

    #[test]
    fn test_reserved_dev_bot_names() {
        assert!(BotDatabaseManager::is_reserved_dev_bot_name("mybot-dev"));
        assert!(BotDatabaseManager::is_reserved_dev_bot_name("dev"));
        assert!(!BotDatabaseManager::is_reserved_dev_bot_name("developer"));
        assert!(!BotDatabaseManager::is_reserved_dev_bot_name("mybot"));
        // Twin naming convention: `{bot}-dev` produces `{bot}_dev` in the DB name.
        assert_eq!(
            BotDatabaseManager::generate_database_name("default", "mybot-dev"),
            "bot_default_mybot_dev"
        );
    }
}