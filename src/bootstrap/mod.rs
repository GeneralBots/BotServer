use crate::config::AppConfig;
use crate::package_manager::{InstallMode, PackageManager};
use anyhow::{Context, Result};
use diesel::prelude::*;
use log::{info, warn};
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct BootstrapManager {
    mode: InstallMode,
    tenant: String,
    base_path: PathBuf,
    config_values: HashMap<String, String>,
}

impl BootstrapManager {
    pub fn new(mode: InstallMode, tenant: Option<String>) -> Self {
        let tenant = tenant.unwrap_or_else(|| "default".to_string());
        let base_path = if mode == InstallMode::Container {
            PathBuf::from("/opt/gbo")
        } else {
            PathBuf::from("./botserver-stack")
        };

        Self {
            mode,
            tenant,
            base_path,
            config_values: HashMap::new(),
        }
    }

    pub fn bootstrap(&mut self) -> Result<AppConfig> {
        info!(
            "Starting bootstrap process in {:?} mode for tenant {}",
            self.mode, self.tenant
        );

        std::fs::create_dir_all(&self.base_path).context("Failed to create base directory")?;

        let pm = PackageManager::new(self.mode.clone(), Some(self.tenant.clone()))?;

        info!("Installing core infrastructure components");
        self.install_and_configure_tables(&pm)?;
        self.install_and_configure_drive(&pm)?;
        self.install_and_configure_cache(&pm)?;
        self.install_and_configure_llm(&pm)?;

        info!("Creating database schema and storing configuration");
        let config = self.build_config()?;
        self.initialize_database(&config)?;
        self.store_configuration_in_db(&config)?;

        info!("Bootstrap completed successfully");
        Ok(config)
    }

    fn install_and_configure_tables(&mut self, pm: &PackageManager) -> Result<()> {
        info!("Installing PostgreSQL tables component");
        pm.install("tables")?;

        let tables_port = self.find_available_port(5432);
        let tables_password = self.generate_password();

        self.config_values
            .insert("TABLES_USERNAME".to_string(), self.tenant.clone());
        self.config_values
            .insert("TABLES_PASSWORD".to_string(), tables_password.clone());
        self.config_values
            .insert("TABLES_SERVER".to_string(), self.get_service_host("tables"));
        self.config_values
            .insert("TABLES_PORT".to_string(), tables_port.to_string());
        self.config_values
            .insert("TABLES_DATABASE".to_string(), format!("{}_db", self.tenant));

        self.wait_for_service(&self.get_service_host("tables"), tables_port, 30)?;

        info!(
            "PostgreSQL configured: {}:{}",
            self.get_service_host("tables"),
            tables_port
        );
        Ok(())
    }

    fn install_and_configure_drive(&mut self, pm: &PackageManager) -> Result<()> {
        info!("Installing MinIO drive component");
        pm.install("drive")?;

        let drive_port = self.find_available_port(9000);
        let _drive_console_port = self.find_available_port(9001);
        let drive_user = "minioadmin".to_string();
        let drive_password = self.generate_password();

        self.config_values.insert(
            "DRIVE_SERVER".to_string(),
            format!("{}:{}", self.get_service_host("drive"), drive_port),
        );
        self.config_values
            .insert("DRIVE_ACCESSKEY".to_string(), drive_user.clone());
        self.config_values
            .insert("DRIVE_SECRET".to_string(), drive_password.clone());
        self.config_values
            .insert("DRIVE_USE_SSL".to_string(), "false".to_string());
        self.config_values
            .insert("DRIVE_ORG_PREFIX".to_string(), self.tenant.clone());
        self.config_values.insert(
            "DRIVE_BUCKET".to_string(),
            format!("{}default.gbai", self.tenant),
        );

        self.wait_for_service(&self.get_service_host("drive"), drive_port, 30)?;

        info!(
            "MinIO configured: {}:{}",
            self.get_service_host("drive"),
            drive_port
        );
        Ok(())
    }

    fn install_and_configure_cache(&mut self, pm: &PackageManager) -> Result<()> {
        info!("Installing Redis cache component");
        pm.install("cache")?;

        let cache_port = self.find_available_port(6379);

        self.config_values.insert(
            "CACHE_URL".to_string(),
            format!("redis://{}:{}/", self.get_service_host("cache"), cache_port),
        );

        self.wait_for_service(&self.get_service_host("cache"), cache_port, 30)?;

        info!(
            "Redis configured: {}:{}",
            self.get_service_host("cache"),
            cache_port
        );
        Ok(())
    }

    fn install_and_configure_llm(&mut self, pm: &PackageManager) -> Result<()> {
        info!("Installing LLM server component");
        pm.install("llm")?;

        let llm_port = self.find_available_port(8081);

        self.config_values.insert(
            "LLM_URL".to_string(),
            format!("http://{}:{}", self.get_service_host("llm"), llm_port),
        );
        self.config_values.insert(
            "AI_ENDPOINT".to_string(),
            format!("http://{}:{}", self.get_service_host("llm"), llm_port),
        );
        self.config_values
            .insert("AI_KEY".to_string(), "empty".to_string());
        self.config_values
            .insert("AI_INSTANCE".to_string(), "llama-local".to_string());
        self.config_values
            .insert("AI_VERSION".to_string(), "1.0".to_string());

        self.wait_for_service(&self.get_service_host("llm"), llm_port, 60)?;

        info!(
            "LLM server configured: {}:{}",
            self.get_service_host("llm"),
            llm_port
        );
        Ok(())
    }

    fn build_config(&self) -> Result<AppConfig> {
        info!("Building application configuration from discovered services");

        let get_str = |key: &str, default: &str| -> String {
            self.config_values
                .get(key)
                .cloned()
                .unwrap_or_else(|| default.to_string())
        };

        let get_u32 = |key: &str, default: u32| -> u32 {
            self.config_values
                .get(key)
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };

        let get_u16 = |key: &str, default: u16| -> u16 {
            self.config_values
                .get(key)
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };

        let get_bool = |key: &str, default: bool| -> bool {
            self.config_values
                .get(key)
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(default)
        };

        let stack_path = self.base_path.clone();

        let database = crate::config::DatabaseConfig {
            username: get_str("TABLES_USERNAME", "botserver"),
            password: get_str("TABLES_PASSWORD", "botserver"),
            server: get_str("TABLES_SERVER", "localhost"),
            port: get_u32("TABLES_PORT", 5432),
            database: get_str("TABLES_DATABASE", "botserver_db"),
        };

        let database_custom = database.clone();

        let minio = crate::config::DriveConfig {
            server: get_str("DRIVE_SERVER", "localhost:9000"),
            access_key: get_str("DRIVE_ACCESSKEY", "minioadmin"),
            secret_key: get_str("DRIVE_SECRET", "minioadmin"),
            use_ssl: get_bool("DRIVE_USE_SSL", false),
            org_prefix: get_str("DRIVE_ORG_PREFIX", "botserver"),
        };

        let email = crate::config::EmailConfig {
            from: get_str("EMAIL_FROM", "noreply@example.com"),
            server: get_str("EMAIL_SERVER", "smtp.example.com"),
            port: get_u16("EMAIL_PORT", 587),
            username: get_str("EMAIL_USER", "user"),
            password: get_str("EMAIL_PASS", "pass"),
        };

        let ai = crate::config::AIConfig {
            instance: get_str("AI_INSTANCE", "llama-local"),
            key: get_str("AI_KEY", "empty"),
            version: get_str("AI_VERSION", "1.0"),
            endpoint: get_str("AI_ENDPOINT", "http://localhost:8081"),
        };

        let server_host = if self.mode == InstallMode::Container {
            "0.0.0.0".to_string()
        } else {
            "127.0.0.1".to_string()
        };

        Ok(AppConfig {
            minio,
            server: crate::config::ServerConfig {
                host: server_host,
                port: self.find_available_port(8080),
            },
            database,
            database_custom,
            email,
            ai,
            s3_bucket: get_str("DRIVE_BUCKET", "default.gbai"),
            site_path: format!("{}/sites", stack_path.display()),
            stack_path,
            db_conn: None,
        })
    }

    fn initialize_database(&self, config: &AppConfig) -> Result<()> {
        use diesel::pg::PgConnection;

        info!("Initializing database schema at {}", config.database_url());

        // Attempt to establish a PostgreSQL connection with retries.
        let mut retries = 5;
        let mut conn = loop {
            match PgConnection::establish(&config.database_url()) {
                Ok(c) => break c,
                Err(e) if retries > 0 => {
                    warn!("Database connection failed, retrying in 2s: {}", e);
                    thread::sleep(Duration::from_secs(2));
                    retries -= 1;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to connect to database after retries: {}",
                        e
                    ))
                }
            }
        };

        // Create the server_configuration table.
        diesel::sql_query(
            "CREATE TABLE IF NOT EXISTS server_configuration (
                id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
                config_key TEXT NOT NULL UNIQUE,
                config_value TEXT NOT NULL,
                config_type TEXT NOT NULL DEFAULT 'string',
                is_encrypted BOOLEAN NOT NULL DEFAULT false,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(&mut conn)
        .context("Failed to create server_configuration table")?;

        // Create the bot_configuration table.
        diesel::sql_query(
            "CREATE TABLE IF NOT EXISTS bot_configuration (
                id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
                bot_id UUID NOT NULL,
                config_key TEXT NOT NULL,
                config_value TEXT NOT NULL,
                config_type TEXT NOT NULL DEFAULT 'string',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(bot_id, config_key)
            )",
        )
        .execute(&mut conn)
        .context("Failed to create bot_configuration table")?;

        // Create the gbot_config_sync table.
        diesel::sql_query(
            "CREATE TABLE IF NOT EXISTS gbot_config_sync (
                id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
                bot_id UUID NOT NULL UNIQUE,
                config_file_path TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                last_sync_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                sync_count INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&mut conn)
        .context("Failed to create gbot_config_sync table")?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    fn store_configuration_in_db(&self, config: &AppConfig) -> Result<()> {
        use diesel::pg::PgConnection;

        info!("Storing configuration in database");

        // Establish a PostgreSQL connection explicitly.
        let mut conn = PgConnection::establish(&config.database_url())
            .context("Failed to establish database connection for storing configuration")?;

        // Store dynamic configuration values.
        for (key, value) in &self.config_values {
            diesel::sql_query(
                "INSERT INTO server_configuration (config_key, config_value, config_type)
                 VALUES ($1, $2, 'string')
                 ON CONFLICT (config_key)
                 DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = NOW()",
            )
            .bind::<diesel::sql_types::Text, _>(key)
            .bind::<diesel::sql_types::Text, _>(value)
            .execute(&mut conn)
            .with_context(|| format!("Failed to store config key: {}", key))?;
        }

        // Store static configuration entries.
        diesel::sql_query(
            "INSERT INTO server_configuration (config_key, config_value, config_type)
             VALUES ('SERVER_HOST', $1, 'string')
             ON CONFLICT (config_key)
             DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = NOW()",
        )
        .bind::<diesel::sql_types::Text, _>(&config.server.host)
        .execute(&mut conn)
        .context("Failed to store SERVER_HOST")?;

        diesel::sql_query(
            "INSERT INTO server_configuration (config_key, config_value, config_type)
             VALUES ('SERVER_PORT', $1, 'string')
             ON CONFLICT (config_key)
             DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = NOW()",
        )
        .bind::<diesel::sql_types::Text, _>(&config.server.port.to_string())
        .execute(&mut conn)
        .context("Failed to store SERVER_PORT")?;

        diesel::sql_query(
            "INSERT INTO server_configuration (config_key, config_value, config_type)
             VALUES ('STACK_PATH', $1, 'string')
             ON CONFLICT (config_key)
             DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = NOW()",
        )
        .bind::<diesel::sql_types::Text, _>(&config.stack_path.display().to_string())
        .execute(&mut conn)
        .context("Failed to store STACK_PATH")?;

        diesel::sql_query(
            "INSERT INTO server_configuration (config_key, config_value, config_type)
             VALUES ('SITES_ROOT', $1, 'string')
             ON CONFLICT (config_key)
             DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = NOW()",
        )
        .bind::<diesel::sql_types::Text, _>(&config.site_path)
        .execute(&mut conn)
        .context("Failed to store SITES_ROOT")?;

        info!(
            "Configuration stored in database successfully with {} entries",
            self.config_values.len() + 4
        );
        Ok(())
    }

    fn find_available_port(&self, preferred: u16) -> u16 {
        if self.mode == InstallMode::Container {
            return preferred;
        }

        for port in preferred..preferred + 100 {
            if TcpListener::bind(("127.0.0.1", port)).is_ok() {
                return port;
            }
        }
        preferred
    }

    fn generate_password(&self) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::rng();
        (0..16)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    fn get_service_host(&self, component: &str) -> String {
        match self.mode {
            InstallMode::Container => {
                let container_name = format!("{}-{}", self.tenant, component);
                self.get_container_ip(&container_name)
                    .unwrap_or_else(|_| "127.0.0.1".to_string())
            }
            InstallMode::Local => "127.0.0.1".to_string(),
        }
    }

    fn get_container_ip(&self, container_name: &str) -> Result<String> {
        let output = Command::new("lxc")
            .args(&["list", container_name, "--format=json"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get container info"));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        if let Some(ip) = json
            .get(0)
            .and_then(|c| c.get("state"))
            .and_then(|s| s.get("network"))
            .and_then(|n| n.get("eth0"))
            .and_then(|e| e.get("addresses"))
            .and_then(|a| a.get(0))
            .and_then(|a| a.get("address"))
            .and_then(|a| a.as_str())
        {
            Ok(ip.to_string())
        } else {
            Err(anyhow::anyhow!("Could not extract container IP"))
        }
    }

    fn wait_for_service(&self, host: &str, port: u16, timeout_secs: u64) -> Result<()> {
        info!(
            "Waiting for service at {}:{} (timeout: {}s)",
            host, port, timeout_secs
        );

        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < timeout_secs {
            if TcpListener::bind((host, port)).is_err() {
                info!("Service {}:{} is ready", host, port);
                return Ok(());
            }
            thread::sleep(Duration::from_secs(1));
        }

        Err(anyhow::anyhow!(
            "Timeout waiting for service at {}:{}",
            host,
            port
        ))
    }
}
