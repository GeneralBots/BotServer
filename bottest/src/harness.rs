use crate::fixtures::{Bot, Customer, Message, QueueEntry, Session, User};
use crate::mocks::{MockLLM, MockZitadel};
use crate::ports::{PortAllocator, TestPorts};
use crate::services::{MinioService, PostgresService, RedisService};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use std::path::PathBuf;
use tokio::sync::OnceCell;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub postgres: bool,
    pub minio: bool,
    pub redis: bool,
    pub mock_zitadel: bool,
    pub mock_llm: bool,
    pub run_migrations: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            postgres: true,
            minio: false,
            redis: false,
            mock_zitadel: true,
            mock_llm: true,
            run_migrations: true,
        }
    }
}

impl TestConfig {
    #[must_use]
    pub const fn minimal() -> Self {
        Self {
            postgres: false,
            minio: false,
            redis: false,
            mock_zitadel: false,
            mock_llm: false,
            run_migrations: false,
        }
    }

    #[must_use]
    pub const fn full() -> Self {
        Self {
            postgres: false,
            minio: false,
            redis: false,
            mock_zitadel: true,
            mock_llm: true,
            run_migrations: false,
        }
    }

    #[must_use]
    pub const fn auto_install() -> Self {
        Self {
            postgres: false,
            minio: false,
            redis: false,
            mock_zitadel: true,
            mock_llm: true,
            run_migrations: false,
        }
    }

    #[must_use]
    pub const fn database_only() -> Self {
        Self {
            postgres: true,
            run_migrations: true,
            ..Self::minimal()
        }
    }

    #[must_use]
    pub const fn use_existing_stack() -> Self {
        Self {
            postgres: false,
            minio: false,
            redis: false,
            mock_zitadel: true,
            mock_llm: true,
            run_migrations: false,
        }
    }
}

pub struct DefaultPorts;

impl DefaultPorts {
    pub const POSTGRES: u16 = 5432;
    pub const MINIO: u16 = 9000;
    pub const REDIS: u16 = 6379;
    pub const ZITADEL: u16 = 8080;
}

pub struct TestContext {
    pub ports: TestPorts,
    pub config: TestConfig,
    pub data_dir: PathBuf,
    pub use_existing_stack: bool,
    test_id: Uuid,
    postgres: Option<PostgresService>,
    minio: Option<MinioService>,
    redis: Option<RedisService>,
    mock_zitadel: Option<MockZitadel>,
    mock_llm: Option<MockLLM>,
    db_pool: OnceCell<DbPool>,
    cleaned_up: bool,
}

impl TestContext {
    pub const fn test_id(&self) -> Uuid {
        self.test_id
    }

    pub fn database_url(&self) -> String {
        if self.use_existing_stack {
            let host = std::env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let port = std::env::var("DB_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(DefaultPorts::POSTGRES);
            let user = std::env::var("DB_USER").unwrap_or_else(|_| "gbuser".to_string());
            let password = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "gbuser".to_string());
            let database = std::env::var("DB_NAME").unwrap_or_else(|_| "botserver".to_string());
            format!("postgres://{user}:{password}@{host}:{port}/{database}")
        } else {
            format!(
                "postgres://bottest:bottest@127.0.0.1:{}/bottest",
                self.ports.postgres
            )
        }
    }

    pub fn minio_endpoint(&self) -> String {
        if self.use_existing_stack {
            format!("http://127.0.0.1:{}", DefaultPorts::MINIO)
        } else {
            format!("http://127.0.0.1:{}", self.ports.minio)
        }
    }

    pub fn redis_url(&self) -> String {
        if self.use_existing_stack {
            format!("redis://127.0.0.1:{}", DefaultPorts::REDIS)
        } else {
            format!("redis://127.0.0.1:{}", self.ports.redis)
        }
    }

    pub fn zitadel_url(&self) -> String {
        if self.use_existing_stack {
            format!("https://127.0.0.1:{}", DefaultPorts::ZITADEL)
        } else {
            format!("http://127.0.0.1:{}", self.ports.mock_zitadel)
        }
    }

    pub fn llm_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.ports.mock_llm)
    }

    pub async fn db_pool(&self) -> Result<&DbPool> {
        self.db_pool
            .get_or_try_init(|| async {
                let manager = ConnectionManager::<PgConnection>::new(self.database_url());
                Pool::builder()
                    .max_size(5)
                    .build(manager)
                    .map_err(|e| anyhow::anyhow!("Failed to create pool: {e}"))
            })
            .await
    }

    pub const fn mock_zitadel(&self) -> Option<&MockZitadel> {
        self.mock_zitadel.as_ref()
    }

    pub const fn mock_llm(&self) -> Option<&MockLLM> {
        self.mock_llm.as_ref()
    }

    pub const fn postgres(&self) -> Option<&PostgresService> {
        self.postgres.as_ref()
    }

    pub const fn minio(&self) -> Option<&MinioService> {
        self.minio.as_ref()
    }

    pub const fn redis(&self) -> Option<&RedisService> {
        self.redis.as_ref()
    }

    pub async fn insert(&self, entity: &dyn Insertable) -> Result<()> {
        let pool = self.db_pool().await?;
        entity.insert(pool)
    }

    pub async fn insert_user(&self, user: &User) -> Result<()> {
        self.insert(user).await
    }

    pub async fn insert_customer(&self, customer: &Customer) -> Result<()> {
        self.insert(customer).await
    }

    pub async fn insert_bot(&self, bot: &Bot) -> Result<()> {
        self.insert(bot).await
    }

    pub async fn insert_session(&self, session: &Session) -> Result<()> {
        self.insert(session).await
    }

    pub async fn insert_message(&self, message: &Message) -> Result<()> {
        self.insert(message).await
    }

    pub async fn insert_queue_entry(&self, entry: &QueueEntry) -> Result<()> {
        self.insert(entry).await
    }

    pub async fn start_botserver(&self) -> Result<BotServerInstance> {
        BotServerInstance::start(self).await
    }

    pub async fn start_botui(&self, botserver_url: &str) -> Result<BotUIInstance> {
        BotUIInstance::start(botserver_url).await
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        if self.cleaned_up {
            return Ok(());
        }

        log::info!("Cleaning up test context {}...", self.test_id);

        if let Some(ref mut pg) = self.postgres {
            let _ = pg.stop().await;
        }

        if let Some(ref mut minio) = self.minio {
            let _ = minio.stop().await;
        }

        if let Some(ref mut redis) = self.redis {
            let _ = redis.stop().await;
        }

        if self.data_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.data_dir);
        }

        self.cleaned_up = true;
        Ok(())
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        log::info!("Dropping test context {}...", self.test_id);

        if let Some(ref mut pg) = self.postgres {
            let _ = pg.cleanup();
        }

        if let Some(ref mut minio) = self.minio {
            let _ = minio.cleanup();
        }

        if let Some(ref mut redis) = self.redis {
            let _ = redis.cleanup();
        }

        if self.data_dir.exists() && !self.cleaned_up {
            let _ = std::fs::remove_dir_all(&self.data_dir);
        }
    }
}

pub trait Insertable: Send + Sync {
    fn insert(&self, pool: &DbPool) -> Result<()>;
}

impl Insertable for User {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO users (id, email, name, role, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO UPDATE SET email = $2, name = $3, role = $4, updated_at = $6",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<Text, _>(&self.email)
        .bind::<Text, _>(&self.name)
        .bind::<Text, _>(format!("{:?}", self.role).to_lowercase())
        .bind::<Timestamptz, _>(self.created_at)
        .bind::<Timestamptz, _>(self.updated_at)
        .execute(&mut conn)?;
        Ok(())
    }
}

impl Insertable for Customer {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Nullable, Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO customers (id, external_id, phone, email, name, channel, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET external_id = $2, phone = $3, email = $4, name = $5, channel = $6, updated_at = $8",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<Text, _>(&self.external_id)
        .bind::<Nullable<Text>, _>(&self.phone)
        .bind::<Nullable<Text>, _>(&self.email)
        .bind::<Nullable<Text>, _>(&self.name)
        .bind::<Text, _>(format!("{:?}", self.channel).to_lowercase())
        .bind::<Timestamptz, _>(self.created_at)
        .bind::<Timestamptz, _>(self.updated_at)
        .execute(&mut conn)?;
        Ok(())
    }
}

impl Insertable for Bot {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Bool, Nullable, Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO bots (id, name, description, kb_enabled, llm_enabled, llm_model, active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id) DO UPDATE SET name = $2, description = $3, kb_enabled = $4, llm_enabled = $5, llm_model = $6, active = $7, updated_at = $9",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<Text, _>(&self.name)
        .bind::<Nullable<Text>, _>(&self.description)
        .bind::<Bool, _>(self.kb_enabled)
        .bind::<Bool, _>(self.llm_enabled)
        .bind::<Nullable<Text>, _>(&self.llm_model)
        .bind::<Bool, _>(self.active)
        .bind::<Timestamptz, _>(self.created_at)
        .bind::<Timestamptz, _>(self.updated_at)
        .execute(&mut conn)?;
        Ok(())
    }
}

impl Insertable for Session {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Nullable, Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO sessions (id, bot_id, customer_id, channel, state, started_at, updated_at, ended_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET state = $5, updated_at = $7, ended_at = $8",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<DieselUuid, _>(self.bot_id)
        .bind::<DieselUuid, _>(self.customer_id)
        .bind::<Text, _>(format!("{:?}", self.channel).to_lowercase())
        .bind::<Text, _>(format!("{:?}", self.state).to_lowercase())
        .bind::<Timestamptz, _>(self.started_at)
        .bind::<Timestamptz, _>(self.updated_at)
        .bind::<Nullable<Timestamptz>, _>(self.ended_at)
        .execute(&mut conn)?;
        Ok(())
    }
}

impl Insertable for Message {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO messages (id, session_id, direction, content, content_type, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<DieselUuid, _>(self.session_id)
        .bind::<Text, _>(format!("{:?}", self.direction).to_lowercase())
        .bind::<Text, _>(&self.content)
        .bind::<Text, _>(format!("{:?}", self.content_type).to_lowercase())
        .bind::<Timestamptz, _>(self.timestamp)
        .execute(&mut conn)?;
        Ok(())
    }
}

impl Insertable for QueueEntry {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Nullable, Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO queue_entries (id, customer_id, session_id, priority, status, entered_at, assigned_at, attendant_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET status = $5, assigned_at = $7, attendant_id = $8",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<DieselUuid, _>(self.customer_id)
        .bind::<DieselUuid, _>(self.session_id)
        .bind::<Text, _>(format!("{:?}", self.priority).to_lowercase())
        .bind::<Text, _>(format!("{:?}", self.status).to_lowercase())
        .bind::<Timestamptz, _>(self.entered_at)
        .bind::<Nullable<Timestamptz>, _>(self.assigned_at)
        .bind::<Nullable<DieselUuid>, _>(self.attendant_id)
        .execute(&mut conn)?;
        Ok(())
    }
}

pub struct BotServerInstance {
    pub url: String,
    pub port: u16,
    pub stack_path: PathBuf,
    process: Option<std::process::Child>,
}

impl BotServerInstance {
    #[must_use]
    pub fn existing(url: &str) -> Self {
        let port = url
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        Self {
            url: url.to_string(),
            port,
            stack_path: PathBuf::from("./botserver-stack"),
            process: None,
        }
    }

    pub async fn start_with_main_stack() -> Result<Self> {
        let port = 8080;
        let url = "https://localhost:9000".to_string();

        let botserver_bin = std::env::var("BOTSERVER_BIN")
            .unwrap_or_else(|_| "../botserver/target/debug/botserver".to_string());

        if !PathBuf::from(&botserver_bin).exists() {
            log::warn!("Botserver binary not found at: {botserver_bin}");
            anyhow::bail!(
                "Botserver binary not found at: {botserver_bin}. Run: cd ../botserver && cargo build"
            );
        }

        let botserver_bin_path =
            std::fs::canonicalize(&botserver_bin).unwrap_or_else(|_| PathBuf::from(&botserver_bin));
        let botserver_dir = botserver_bin_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| {
                std::fs::canonicalize("../botserver")
                    .unwrap_or_else(|_| PathBuf::from("../botserver"))
            });

        let stack_path = botserver_dir.join("botserver-stack");

        if !stack_path.exists() {
            anyhow::bail!(
                "Main botserver-stack not found at {}.\n\
                 Run botserver once to initialize: cd ../botserver && cargo run",
                stack_path.display()
            );
        }

        log::info!(
            "Starting botserver with MAIN stack at {}",
            stack_path.display()
        );
        println!("🚀 Starting BotServer with main stack...");
        println!("   Stack: {}", stack_path.display());

        let process = std::process::Command::new(&botserver_bin_path)
            .current_dir(&botserver_dir)
            .arg("--noconsole")
            .env_remove("RUST_LOG")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .ok();

        if process.is_some() {
            let max_wait = 120;
            log::info!("Waiting for botserver to start (max {max_wait}s)...");

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default();

            for i in 0..max_wait {
                if let Ok(resp) = client.get(format!("{url}/health")).send().await {
                    if resp.status().is_success() {
                        log::info!("Botserver ready on port {port}");
                        println!("   ✓ BotServer ready at {url}");
                        return Ok(Self {
                            url,
                            port,
                            stack_path,
                            process,
                        });
                    }
                }
                if i % 10 == 0 && i > 0 {
                    log::info!("Still waiting for botserver... ({i}s)");
                    println!("   ... waiting ({i}s)");
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            log::warn!("Botserver did not respond in time");
            println!("   ⚠ Botserver may not be ready");
        }

        Ok(Self {
            url,
            port,
            stack_path,
            process,
        })
    }
}

pub struct BotUIInstance {
    pub url: String,
    pub port: u16,
    process: Option<std::process::Child>,
}

impl BotUIInstance {
    #[must_use]
    pub fn existing(url: &str) -> Self {
        let port = url
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);
        Self {
            url: url.to_string(),
            port,
            process: None,
        }
    }
}

impl BotUIInstance {
    pub async fn start(botserver_url: &str) -> Result<Self> {
        let port = crate::ports::PortAllocator::allocate();
        let url = format!("http://127.0.0.1:{port}");

        let botui_bin = std::env::var("BOTUI_BIN")
            .unwrap_or_else(|_| "../botui/target/debug/botui".to_string());

        if !PathBuf::from(&botui_bin).exists() {
            log::warn!("BotUI binary not found at: {botui_bin}");
            return Ok(Self {
                url,
                port,
                process: None,
            });
        }

        let botui_bin_path =
            std::fs::canonicalize(&botui_bin).unwrap_or_else(|_| PathBuf::from(&botui_bin));
        let botui_dir = botui_bin_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| {
                std::fs::canonicalize("../botui").unwrap_or_else(|_| PathBuf::from("../botui"))
            });

        log::info!("Starting botui from: {botui_bin} on port {port}");
        log::info!("  BOTUI_PORT={port}");
        log::info!("  BOTSERVER_URL={botserver_url}");
        log::info!("  Working directory: {}", botui_dir.display());

        let process = std::process::Command::new(&botui_bin_path)
            .current_dir(&botui_dir)
            .env("BOTUI_PORT", port.to_string())
            .env("BOTSERVER_URL", botserver_url)
            .env_remove("RUST_LOG")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .ok();

        if process.is_some() {
            let max_wait = 30;
            log::info!("Waiting for botui to become ready... (max {max_wait}s)");
            for i in 0..max_wait {
                if let Ok(resp) = reqwest::get(&format!("{url}/health")).await {
                    if resp.status().is_success() {
                        log::info!("BotUI is ready on port {port}");
                        return Ok(Self { url, port, process });
                    }
                }
                if let Ok(resp) = reqwest::get(&url).await {
                    if resp.status().is_success() {
                        log::info!("BotUI is ready on port {port}");
                        return Ok(Self { url, port, process });
                    }
                }
                if i % 5 == 0 {
                    log::info!("Still waiting for botui... ({i}s)");
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            log::warn!("BotUI did not respond in time");
        }

        Ok(Self {
            url,
            port,
            process: None,
        })
    }

    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.process.is_some()
    }
}

impl Drop for BotUIInstance {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl BotServerInstance {
    pub async fn start(ctx: &TestContext) -> Result<Self> {
        let port = ctx.ports.botserver;
        let url = format!("http://127.0.0.1:{port}");

        let stack_path = ctx.data_dir.join("botserver-stack");
        std::fs::create_dir_all(&stack_path)?;
        let stack_path = stack_path.canonicalize().unwrap_or(stack_path);
        log::info!("Created clean test stack at: {}", stack_path.display());

        let botserver_bin = std::env::var("BOTSERVER_BIN")
            .unwrap_or_else(|_| default_botserver_bin());

        if !PathBuf::from(&botserver_bin).exists() {
            log::warn!("Botserver binary not found at: {botserver_bin}");
            return Ok(Self {
                url,
                port,
                stack_path,
                process: None,
            });
        }

        log::info!("Starting botserver from: {botserver_bin}");

        let botserver_bin_path =
            std::fs::canonicalize(&botserver_bin).unwrap_or_else(|_| PathBuf::from(&botserver_bin));
        let botserver_dir = botserver_bin_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
            .filter(|p| p.join("botserver-installers").exists())
            .unwrap_or_else(|| {
                std::fs::canonicalize("../botserver")
                    .unwrap_or_else(|_| PathBuf::from("../botserver"))
            });

        log::info!("Botserver working directory: {}", botserver_dir.display());
        log::info!("Stack path (absolute): {}", stack_path.display());

        let installers_path = botserver_dir.join("botserver-installers");
        let installers_path = installers_path.canonicalize().unwrap_or(installers_path);
        log::info!("Using installers from: {}", installers_path.display());

        let process = std::process::Command::new(&botserver_bin_path)
            .current_dir(&botserver_dir)
            .arg("--stack-path")
            .arg(&stack_path)
            .arg("--port")
            .arg(port.to_string())
            .arg("--noconsole")
            .env_remove("RUST_LOG")
            .env("GBO_NO_DOTENV", "1")
            .env_remove("VAULT_ADDR")
            .env_remove("VAULT_TOKEN")
            .env_remove("VAULT_CACERT")
            .env_remove("VAULT_SKIP_VERIFY")
            .env_remove("VAULT_CLIENT_CERT")
            .env_remove("VAULT_CLIENT_KEY")
            .env_remove("TABLES_HOST")
            .env_remove("TABLES_PORT")
            .env_remove("TABLES_DATABASE")
            .env_remove("TABLES_USERNAME")
            .env_remove("TABLES_PASSWORD")
            .env("BOTSERVER_INSTALLERS_PATH", &installers_path)
            .env("DATABASE_URL", ctx.database_url())
            .env("DIRECTORY_URL", ctx.zitadel_url())
            .env("ZITADEL_CLIENT_ID", "test-client-id")
            .env("ZITADEL_CLIENT_SECRET", "test-client-secret")
            .env("DRIVE_ACCESSKEY", "minioadmin")
            .env("DRIVE_SECRET", "minioadmin")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .ok();

        if process.is_some() {
            let max_wait = 600;
            log::info!("Waiting for botserver to bootstrap and become ready... (max {max_wait}s)");
            for i in 0..max_wait {
                if let Ok(resp) = reqwest::get(&format!("{url}/health")).await {
                    if resp.status().is_success() {
                        log::info!("Botserver is ready on port {port}");
                        return Ok(Self {
                            url,
                            port,
                            stack_path,
                            process,
                        });
                    }
                }
                if i % 10 == 0 {
                    log::info!("Still waiting for botserver... ({i}s)");
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            log::warn!("Botserver did not respond to health check in time");
        }

        Ok(Self {
            url,
            port,
            stack_path,
            process: None,
        })
    }

    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.process.is_some()
    }

    
    }

impl Drop for BotServerInstance {
    fn drop(&mut self) {
        if let Some(ref mut process) = self.process {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

fn default_botserver_bin() -> String {
    let candidates = ["../target/debug/botserver", "../botserver/target/debug/botserver"];
    for candidate in candidates {
        if PathBuf::from(candidate).exists() {
            return candidate.to_string();
        }
    }
    "../botserver/target/debug/botserver".to_string()
}

pub struct TestHarness;

impl TestHarness {
    pub async fn setup(config: TestConfig) -> Result<TestContext> {
        Self::setup_internal(config, false).await
    }

    pub async fn with_existing_stack() -> Result<TestContext> {
        Self::setup_internal(TestConfig::use_existing_stack(), true).await
    }

    fn cleanup_existing_processes() {
        log::info!("Cleaning up any existing stack processes before test...");

        let patterns = [
            "botserver",
            "botui",
            "vault",
            "postgres",
            "zitadel",
            "minio",
            "llama-server",
            "valkey-server",
            "valkey",
            "chromedriver",
            "chrome.*--user-data-dir=/tmp/browser-test",
            "brave.*--user-data-dir=/tmp/browser-test",
        ];

        for pattern in patterns {
            let _ = std::process::Command::new("pkill")
                .args(["-9", "-f", pattern])
                .output();
        }

        let _ = std::process::Command::new("rm")
            .args(["-rf", "/tmp/browser-test-*"])
            .output();

        let _ = std::process::Command::new("sh")
            .args(["-c", "find ./tmp -maxdepth 1 -name 'bottest-*' -type d -mmin +60 -exec rm -rf {} + 2>/dev/null"])
            .output();

        std::thread::sleep(std::time::Duration::from_millis(1000));

        log::info!("Process cleanup completed");
    }

    async fn setup_internal(config: TestConfig, use_existing_stack: bool) -> Result<TestContext> {
        let _ = env_logger::builder().is_test(true).try_init();

        if !use_existing_stack {
            Self::cleanup_existing_processes();
        }

        let test_id = Uuid::new_v4();
        let data_dir = PathBuf::from("./tmp").join(format!("bottest-{test_id}"));

        std::fs::create_dir_all(&data_dir)?;

        let ports = if use_existing_stack {
            TestPorts {
                postgres: DefaultPorts::POSTGRES,
                minio: DefaultPorts::MINIO,
                redis: DefaultPorts::REDIS,
                botserver: PortAllocator::allocate(),
                mock_zitadel: PortAllocator::allocate(),
                mock_llm: PortAllocator::allocate(),
            }
        } else {
            TestPorts::allocate()
        };

        log::info!(
            "Test {test_id} allocated ports: {ports:?}, data_dir: {}, use_existing_stack: {use_existing_stack}",
            data_dir.display()
        );

        let data_dir_str = data_dir.to_str().unwrap().to_string();

        let mut ctx = TestContext {
            ports,
            config: config.clone(),
            data_dir,
            use_existing_stack,
            test_id,
            postgres: None,
            minio: None,
            redis: None,
            mock_zitadel: None,
            mock_llm: None,
            db_pool: OnceCell::new(),
            cleaned_up: false,
        };

        if config.postgres {
            log::info!("Starting PostgreSQL on port {}...", ctx.ports.postgres);
            let pg = PostgresService::start(ctx.ports.postgres, &data_dir_str).await?;
            if config.run_migrations {
                pg.run_migrations()?;
            }
            ctx.postgres = Some(pg);
        }

        if config.minio {
            log::info!("Starting MinIO on port {}...", ctx.ports.minio);
            ctx.minio = Some(MinioService::start(ctx.ports.minio, &data_dir_str).await?);
        }

        if config.redis {
            log::info!("Starting Redis on port {}...", ctx.ports.redis);
            ctx.redis = Some(RedisService::start(ctx.ports.redis, &data_dir_str).await?);
        }

        if config.mock_zitadel {
            log::info!(
                "Starting mock Zitadel on port {}...",
                ctx.ports.mock_zitadel
            );
            ctx.mock_zitadel = Some(MockZitadel::start(ctx.ports.mock_zitadel).await?);
        }

        if config.mock_llm {
            log::info!("Starting mock LLM on port {}...", ctx.ports.mock_llm);
            ctx.mock_llm = Some(MockLLM::start(ctx.ports.mock_llm).await?);
        }

        Ok(ctx)
    }

    pub async fn quick() -> Result<TestContext> {
        Self::setup(TestConfig::default()).await
    }

    pub async fn full() -> Result<TestContext> {
        if std::env::var("FRESH_STACK").is_ok() {
            Self::setup(TestConfig::full()).await
        } else {
            Self::with_existing_stack().await
        }
    }

    pub async fn with_auto_install() -> Result<TestContext> {
        Self::setup(TestConfig::auto_install()).await
    }

    pub async fn minimal() -> Result<TestContext> {
        Self::setup(TestConfig::minimal()).await
    }

    pub async fn database_only() -> Result<TestContext> {
        Self::setup(TestConfig::database_only()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_minimal_harness() {
        let ctx = TestHarness::minimal().await.unwrap();
        assert!(ctx.ports.postgres >= 15000);
        assert!(ctx.data_dir.to_str().unwrap().contains("bottest-"));
    }

    #[test]
    fn test_config_default() {
        let config = TestConfig::default();
        assert!(config.postgres);
        assert!(!config.minio);
        assert!(!config.redis);
        assert!(config.mock_zitadel);
        assert!(config.mock_llm);
        assert!(config.run_migrations);
    }

    #[test]
    fn test_config_full() {
        let config = TestConfig::full();
        assert!(!config.postgres);
        assert!(!config.minio);
        assert!(!config.redis);
        assert!(config.mock_zitadel);
        assert!(config.mock_llm);
        assert!(!config.run_migrations);
    }

    #[test]
    fn test_config_minimal() {
        let config = TestConfig::minimal();
        assert!(!config.postgres);
        assert!(!config.minio);
        assert!(!config.redis);
        assert!(!config.mock_zitadel);
        assert!(!config.mock_llm);
        assert!(!config.run_migrations);
    }

    #[test]
    fn test_config_database_only() {
        let config = TestConfig::database_only();
        assert!(config.postgres);
        assert!(!config.minio);
        assert!(!config.redis);
        assert!(!config.mock_zitadel);
        assert!(!config.mock_llm);
        assert!(config.run_migrations);
    }
}
