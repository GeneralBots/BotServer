use diesel::prelude::*;
use diesel::sql_types::Text;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Application configuration - reads from database instead of .env
#[derive(Clone)]
pub struct AppConfig {
    pub minio: DriveConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub database_custom: DatabaseConfig,
    pub email: EmailConfig,
    pub ai: AIConfig,
    pub site_path: String,
    pub s3_bucket: String,
    pub stack_path: PathBuf,
    pub(crate) db_conn: Option<Arc<Mutex<PgConnection>>>,
}

#[derive(Clone)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
    pub server: String,
    pub port: u32,
    pub database: String,
}

#[derive(Clone)]
pub struct DriveConfig {
    pub server: String,
    pub access_key: String,
    pub secret_key: String,
    pub use_ssl: bool,
    pub org_prefix: String,
}

#[derive(Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Clone)]
pub struct EmailConfig {
    pub from: String,
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub struct AIConfig {
    pub instance: String,
    pub key: String,
    pub version: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, QueryableByName)]
pub struct ServerConfigRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub config_key: String,
    #[diesel(sql_type = Text)]
    pub config_value: String,
    #[diesel(sql_type = Text)]
    pub config_type: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_encrypted: bool,
}

impl AppConfig {
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.username,
            self.database.password,
            self.database.server,
            self.database.port,
            self.database.database
        )
    }

    pub fn database_custom_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database_custom.username,
            self.database_custom.password,
            self.database_custom.server,
            self.database_custom.port,
            self.database_custom.database
        )
    }

    /// Get stack path for a specific component
    pub fn component_path(&self, component: &str) -> PathBuf {
        self.stack_path.join(component)
    }

    /// Get binary path for a component
    pub fn bin_path(&self, component: &str) -> PathBuf {
        self.stack_path.join("bin").join(component)
    }

    /// Get data path for a component
    pub fn data_path(&self, component: &str) -> PathBuf {
        self.stack_path.join("data").join(component)
    }

    /// Get config path for a component
    pub fn config_path(&self, component: &str) -> PathBuf {
        self.stack_path.join("conf").join(component)
    }

    /// Get log path for a component
    pub fn log_path(&self, component: &str) -> PathBuf {
        self.stack_path.join("logs").join(component)
    }

    /// Load configuration from database
    /// Falls back to defaults if database is not yet initialized
    pub fn from_database(conn: &mut PgConnection) -> Self {
        info!("Loading configuration from database...");

        // Load all configuration from database
        let config_map = match Self::load_config_from_db(conn) {
            Ok(map) => {
                info!(
                    "Successfully loaded {} config values from database",
                    map.len()
                );
                map
            }
            Err(e) => {
                warn!(
                    "Failed to load config from database: {}. Using defaults.",
                    e
                );
                HashMap::new()
            }
        };

        // Helper to get config value with fallback
        let get_str = |key: &str, default: &str| -> String {
            config_map
                .get(key)
                .map(|v| v.config_value.clone())
                .unwrap_or_else(|| default.to_string())
        };

        let get_u32 = |key: &str, default: u32| -> u32 {
            config_map
                .get(key)
                .and_then(|v| v.config_value.parse().ok())
                .unwrap_or(default)
        };

        let get_u16 = |key: &str, default: u16| -> u16 {
            config_map
                .get(key)
                .and_then(|v| v.config_value.parse().ok())
                .unwrap_or(default)
        };

        let get_bool = |key: &str, default: bool| -> bool {
            config_map
                .get(key)
                .map(|v| v.config_value.to_lowercase() == "true")
                .unwrap_or(default)
        };

        let stack_path = PathBuf::from(get_str("STACK_PATH", "./botserver-stack"));

        let database = DatabaseConfig {
            username: get_str("TABLES_USERNAME", "botserver"),
            password: get_str("TABLES_PASSWORD", "botserver"),
            server: get_str("TABLES_SERVER", "localhost"),
            port: get_u32("TABLES_PORT", 5432),
            database: get_str("TABLES_DATABASE", "botserver"),
        };

        let database_custom = DatabaseConfig {
            username: get_str("CUSTOM_USERNAME", "user"),
            password: get_str("CUSTOM_PASSWORD", "pass"),
            server: get_str("CUSTOM_SERVER", "localhost"),
            port: get_u32("CUSTOM_PORT", 5432),
            database: get_str("CUSTOM_DATABASE", "custom"),
        };

        let minio = DriveConfig {
            server: get_str("DRIVE_SERVER", "localhost:9000"),
            access_key: get_str("DRIVE_ACCESSKEY", "minioadmin"),
            secret_key: get_str("DRIVE_SECRET", "minioadmin"),
            use_ssl: get_bool("DRIVE_USE_SSL", false),
            org_prefix: get_str("DRIVE_ORG_PREFIX", "botserver"),
        };

        let email = EmailConfig {
            from: get_str("EMAIL_FROM", "noreply@example.com"),
            server: get_str("EMAIL_SERVER", "smtp.example.com"),
            port: get_u16("EMAIL_PORT", 587),
            username: get_str("EMAIL_USER", "user"),
            password: get_str("EMAIL_PASS", "pass"),
        };

        let ai = AIConfig {
            instance: get_str("AI_INSTANCE", "gpt-4"),
            key: get_str("AI_KEY", ""),
            version: get_str("AI_VERSION", "2023-12-01-preview"),
            endpoint: get_str("AI_ENDPOINT", "https://api.openai.com"),
        };

        AppConfig {
            minio,
            server: ServerConfig {
                host: get_str("SERVER_HOST", "127.0.0.1"),
                port: get_u16("SERVER_PORT", 8080),
            },
            database,
            database_custom,
            email,
            ai,
            s3_bucket: get_str("DRIVE_BUCKET", "default"),
            site_path: get_str("SITES_ROOT", "./botserver-stack/sites"),
            stack_path,
            db_conn: None,
        }
    }

    /// Legacy method - reads from .env for backward compatibility
    /// Will be deprecated once database setup is complete
    pub fn from_env() -> Self {
        warn!("Loading configuration from environment variables (legacy mode)");

        let stack_path =
            std::env::var("STACK_PATH").unwrap_or_else(|_| "./botserver-stack".to_string());

        let database = DatabaseConfig {
            username: std::env::var("TABLES_USERNAME").unwrap_or_else(|_| "botserver".to_string()),
            password: std::env::var("TABLES_PASSWORD").unwrap_or_else(|_| "botserver".to_string()),
            server: std::env::var("TABLES_SERVER").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("TABLES_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5432),
            database: std::env::var("TABLES_DATABASE").unwrap_or_else(|_| "botserver".to_string()),
        };

        let database_custom = DatabaseConfig {
            username: std::env::var("CUSTOM_USERNAME").unwrap_or_else(|_| "user".to_string()),
            password: std::env::var("CUSTOM_PASSWORD").unwrap_or_else(|_| "pass".to_string()),
            server: std::env::var("CUSTOM_SERVER").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("CUSTOM_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5432),
            database: std::env::var("CUSTOM_DATABASE").unwrap_or_else(|_| "custom".to_string()),
        };

        let minio = DriveConfig {
            server: std::env::var("DRIVE_SERVER").unwrap_or_else(|_| "localhost:9000".to_string()),
            access_key: std::env::var("DRIVE_ACCESSKEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            secret_key: std::env::var("DRIVE_SECRET").unwrap_or_else(|_| "minioadmin".to_string()),
            use_ssl: std::env::var("DRIVE_USE_SSL")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            org_prefix: std::env::var("DRIVE_ORG_PREFIX")
                .unwrap_or_else(|_| "botserver".to_string()),
        };

        let email = EmailConfig {
            from: std::env::var("EMAIL_FROM").unwrap_or_else(|_| "noreply@example.com".to_string()),
            server: std::env::var("EMAIL_SERVER")
                .unwrap_or_else(|_| "smtp.example.com".to_string()),
            port: std::env::var("EMAIL_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            username: std::env::var("EMAIL_USER").unwrap_or_else(|_| "user".to_string()),
            password: std::env::var("EMAIL_PASS").unwrap_or_else(|_| "pass".to_string()),
        };

        let ai = AIConfig {
            instance: std::env::var("AI_INSTANCE").unwrap_or_else(|_| "gpt-4".to_string()),
            key: std::env::var("AI_KEY").unwrap_or_else(|_| "".to_string()),
            version: std::env::var("AI_VERSION")
                .unwrap_or_else(|_| "2023-12-01-preview".to_string()),
            endpoint: std::env::var("AI_ENDPOINT")
                .unwrap_or_else(|_| "https://api.openai.com".to_string()),
        };

        AppConfig {
            minio,
            server: ServerConfig {
                host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: std::env::var("SERVER_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(8080),
            },
            database,
            database_custom,
            email,
            ai,
            s3_bucket: std::env::var("DRIVE_BUCKET").unwrap_or_else(|_| "default".to_string()),
            site_path: std::env::var("SITES_ROOT")
                .unwrap_or_else(|_| "./botserver-stack/sites".to_string()),
            stack_path: PathBuf::from(stack_path),
            db_conn: None,
        }
    }

    /// Load all configuration from database into a HashMap
    fn load_config_from_db(
        conn: &mut PgConnection,
    ) -> Result<HashMap<String, ServerConfigRow>, diesel::result::Error> {
        // Try to query the server_configuration table
        let results = diesel::sql_query(
            "SELECT id, config_key, config_value, config_type, is_encrypted
             FROM server_configuration",
        )
        .load::<ServerConfigRow>(conn)?;

        let mut map = HashMap::new();
        for row in results {
            map.insert(row.config_key.clone(), row);
        }

        Ok(map)
    }

    /// Update a configuration value in the database
    pub fn set_config(
        &self,
        conn: &mut PgConnection,
        key: &str,
        value: &str,
    ) -> Result<(), diesel::result::Error> {
        diesel::sql_query("SELECT set_config($1, $2)")
            .bind::<Text, _>(key)
            .bind::<Text, _>(value)
            .execute(conn)?;

        info!("Updated configuration: {} = {}", key, value);
        Ok(())
    }

    /// Get a configuration value from the database
    pub fn get_config(
        &self,
        conn: &mut PgConnection,
        key: &str,
        fallback: Option<&str>,
    ) -> Result<String, diesel::result::Error> {
        // Use empty string when no fallback is supplied
        let fallback_str = fallback.unwrap_or("");

        // Define a temporary struct that matches the shape of the query result.
        #[derive(Debug, QueryableByName)]
        struct ConfigValue {
            #[diesel(sql_type = Text)]
            value: String,
        }

        // Execute the query and map the resulting row to the inner string.
        let result = diesel::sql_query("SELECT get_config($1, $2) as value")
            .bind::<Text, _>(key)
            .bind::<Text, _>(fallback_str)
            .get_result::<ConfigValue>(conn)
            .map(|row| row.value)?;

        Ok(result)
    }
}

/// Configuration manager for handling .gbot/config.csv files
pub struct ConfigManager {
    conn: Arc<Mutex<PgConnection>>,
}

impl ConfigManager {
    pub fn new(conn: Arc<Mutex<PgConnection>>) -> Self {
        Self { conn }
    }

    /// Watch and sync .gbot/config.csv file for a bot
    pub fn sync_gbot_config(
        &self,
        bot_id: &uuid::Uuid,
        config_path: &str,
    ) -> Result<usize, String> {
        // Import necessary crates for hashing and file handling
        use sha2::{Digest, Sha256};
        use std::fs;

        // Read the config.csv file
        let content = fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        // Calculate file hash
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let file_hash = format!("{:x}", hasher.finalize());

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        // Check if file has changed
        #[derive(QueryableByName)]
        struct SyncHash {
            #[diesel(sql_type = Text)]
            file_hash: String,
        }

        let last_hash: Option<String> =
            diesel::sql_query("SELECT file_hash FROM gbot_config_sync WHERE bot_id = $1")
                .bind::<diesel::sql_types::Uuid, _>(bot_id)
                .get_result::<SyncHash>(&mut *conn)
                .optional()
                .map_err(|e| format!("Database error: {}", e))?
                .map(|row| row.file_hash);

        if last_hash.as_ref() == Some(&file_hash) {
            info!("Config file unchanged for bot {}", bot_id);
            return Ok(0);
        }

        // Parse CSV and update bot configuration
        let mut updated = 0;
        for line in content.lines().skip(1) {
            // Skip header
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let key = parts[0].trim();
                let value = parts[1].trim();

                // Insert or update bot configuration
                diesel::sql_query(
                    "INSERT INTO bot_configuration (id, bot_id, config_key, config_value, config_type)
                     VALUES (gen_random_uuid()::text, $1, $2, $3, 'string')
                     ON CONFLICT (bot_id, config_key)
                     DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = NOW()"
                )
                .bind::<diesel::sql_types::Uuid, _>(bot_id)
                .bind::<diesel::sql_types::Text, _>(key)
                .bind::<diesel::sql_types::Text, _>(value)
                .execute(&mut *conn)
                .map_err(|e| format!("Failed to update config: {}", e))?;

                updated += 1;
            }
        }

        // Update sync record
        diesel::sql_query(
            "INSERT INTO gbot_config_sync (id, bot_id, config_file_path, file_hash, sync_count)
             VALUES (gen_random_uuid()::text, $1, $2, $3, 1)
             ON CONFLICT (bot_id)
             DO UPDATE SET last_sync_at = NOW(), file_hash = EXCLUDED.file_hash,
                          sync_count = gbot_config_sync.sync_count + 1",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::Text, _>(config_path)
        .bind::<diesel::sql_types::Text, _>(&file_hash)
        .execute(&mut *conn)
        .map_err(|e| format!("Failed to update sync record: {}", e))?;

        info!(
            "Synced {} config values for bot {} from {}",
            updated, bot_id, config_path
        );
        Ok(updated)
    }
}
