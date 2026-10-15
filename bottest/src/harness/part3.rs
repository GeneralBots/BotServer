//! Split from `harness.rs` — 290 lines.

use super::*;

impl BotServerInstance {
    pub async fn start(ctx: &TestContext) -> Result<Self> {
        let port = ctx.ports.botserver;
        let url = format!("http://127.0.0.1:{port}");

        let stack_path = ctx.data_dir.join("botserver-stack");
        std::fs::create_dir_all(&stack_path)?;
        let stack_path = stack_path.canonicalize().unwrap_or(stack_path);
        log::info!("Created clean test stack at: {}", stack_path.display());

        let botserver_bin =
            std::env::var("BOTSERVER_BIN").unwrap_or_else(|_| default_botserver_bin());

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

pub(crate) fn default_botserver_bin() -> String {
    let candidates = [
        "../target/debug/botserver",
        "../botserver/target/debug/botserver",
    ];
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

    pub(crate) fn cleanup_existing_processes() {
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

    pub(crate) async fn setup_internal(
        config: TestConfig,
        use_existing_stack: bool,
    ) -> Result<TestContext> {
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
