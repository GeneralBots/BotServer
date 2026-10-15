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

mod part2;
pub use part2::*;
mod part3;
pub use part3::*;

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
