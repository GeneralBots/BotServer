//! Bootstrap and application initialization logic

use diesel::RunQueryDsl;
use botcore::shared::utils::{create_conn, get_stack_path};
use crate::core::config::AppConfig;
use crate::core::i18n;
use crate::core::package_manager::InstallMode;
use log::{error, info, trace, warn};
use super::BootstrapProgress;
use super::migrations::run_diesel_migrations;
use std::path::Path;
use std::process::Command;

pub fn init_logging_and_i18n(no_console: bool, no_ui: bool) {

    if no_console || no_ui {
        botlib::logging::init_compact_logger_with_style("info");
        println!(r#"
  ╔═══════════════════════════════════════╗
  ║         General Bots v{}          ║
  ╚═══════════════════════════════════════╝
"#, env!("CARGO_PKG_VERSION"));
    }

    let locales_path = if std::path::Path::new("./locales").exists() {
        "./locales"
    } else if std::path::Path::new("../botlib/locales").exists() {
        "../botlib/locales"
    } else if std::path::Path::new("../locales").exists() {
        "../locales"
    } else {
        "./locales"
    };
    if let Err(e) = i18n::init_i18n(locales_path) {
        warn!(
            "Failed to initialize i18n from {}: {}. Translations will show keys.",
            locales_path, e
        );
    } else {
        info!(
            "i18n initialized from {} with locales: {:?}",
            locales_path,
            i18n::available_locales()
        );
    }
}
pub fn parse_cli_args(args: &[String]) -> (InstallMode, Option<String>) {
    let install_mode = if args.contains(&"--container".to_string()) {
        InstallMode::Container
    } else {
        InstallMode::Local
    };

    let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") {
        args.get(idx + 1).cloned()
    } else {
        None
    };

    (install_mode, tenant)
}
pub async fn run_bootstrap(
    install_mode: InstallMode,
    tenant: Option<String>,
    progress_tx: &tokio::sync::mpsc::UnboundedSender<BootstrapProgress>,
) -> Result<AppConfig, std::io::Error> {
    use crate::core::bootstrap::BootstrapManager;

    trace!("Starting bootstrap process...");
    let progress_tx_clone = progress_tx.clone();
    let cfg = {
        progress_tx_clone
            .send(BootstrapProgress::StartingBootstrap)
            .ok();

        trace!("Creating BootstrapManager...");
        let mut bootstrap = BootstrapManager::new(install_mode, tenant);

        let stack_path = get_stack_path();
        let env_path = std::path::Path::new("./.env");
        let stack_env_path_str = format!("{}/.env", stack_path);
        let vault_init_path_str = format!("{}/conf/vault/init.json", stack_path);
        let stack_env_path = std::path::Path::new(&stack_env_path_str);
        let vault_init_path = std::path::Path::new(&vault_init_path_str);
        let env_exists = env_path.exists();
        let stack_env_exists = stack_env_path.exists();
        let vault_init_exists = vault_init_path.exists();

        // If VAULT_ADDR points to a remote server, treat bootstrap as completed
        // All services are assumed to be running in separate containers
        let vault_addr = std::env::var("VAULT_ADDR").unwrap_or_default();
        let is_remote_vault = !vault_addr.is_empty()
            && !vault_addr.contains("localhost")
            && !vault_addr.contains("127.0.0.1");

        let bootstrap_completed = is_remote_vault || ((env_exists || stack_env_exists) && vault_init_exists);

        info!(
            "Bootstrap check: .env exists={}, stack/.env exists={}, init.json exists={}, remote_vault={}, bootstrap_completed={}",
            env_exists, stack_env_exists, vault_init_exists, is_remote_vault, bootstrap_completed
        );

        let cfg = if bootstrap_completed {
            info!(">>> BRANCH: bootstrap_completed=TRUE - starting services only");
            trace!("Services already configured, ensuring all are running...");
            info!("Ensuring database and drive services are running...");
            progress_tx_clone
                .send(BootstrapProgress::StartingComponent(
                    "all services".to_string(),
                ))
                .ok();
            trace!("Calling bootstrap.start_all()...");
            bootstrap.start_all().await.map_err(std::io::Error::other)?;
            trace!("bootstrap.start_all() completed");

            trace!("Connecting to database...");
            progress_tx_clone
                .send(BootstrapProgress::ConnectingDatabase)
                .ok();

            trace!("Creating database connection...");
            match create_conn() {
                Ok(pool) => {
                    trace!("Database connection successful, loading config from database");
                    AppConfig::from_database(&pool).unwrap_or_else(|e| {
                        warn!("Failed to load config from database: {}, trying env", e);
                        AppConfig::from_env().unwrap_or_else(|env_e| {
                            error!("Failed to load config from env: {}", env_e);
                            AppConfig::default()
                        })
                    })
                }
                Err(e) => {
                    trace!(
                        "Database connection failed: {:?}, loading config from env",
                        e
                    );
                    AppConfig::from_env().unwrap_or_else(|e| {
                        error!("Failed to load config from env: {}", e);
                        AppConfig::default()
                    })
                }
            }
        } else {
            info!(">>> BRANCH: bootstrap_completed=FALSE - running full bootstrap");
            info!("Bootstrap not complete - running full bootstrap...");
            trace!(".env file not found, running bootstrap.bootstrap()...");
            if let Err(e) = bootstrap.bootstrap().await {
                error!("Bootstrap failed: {}", e);
                return Err(std::io::Error::other(format!("Bootstrap failed: {e}")));
            }
            trace!("bootstrap.bootstrap() completed");
            progress_tx_clone
                .send(BootstrapProgress::StartingComponent(
                    "all services".to_string(),
                ))
                .ok();
            bootstrap.start_all().await.map_err(std::io::Error::other)?;

            match create_conn() {
                Ok(pool) => AppConfig::from_database(&pool).unwrap_or_else(|e| {
                    warn!("Failed to load config from database: {}, trying env", e);
                    AppConfig::from_env().unwrap_or_else(|env_e| {
                        error!("Failed to load config from env: {}", env_e);
                        AppConfig::default()
                    })
                }),
                Err(_) => AppConfig::from_env().unwrap_or_else(|e| {
                    error!("Failed to load config from env: {}", e);
                    AppConfig::default()
                }),
            }
        };

        trace!("Ensuring bot templates repository is up to date...");
        ensure_templates_repo().await;

        trace!("Config loaded, syncing templates to database...");
        progress_tx_clone
            .send(BootstrapProgress::UploadingTemplates)
            .ok();

        if let Err(e) = bootstrap.sync_templates_to_database() {
            warn!("Failed to sync templates to database: {}", e);
        } else {
            trace!("Templates synced to database");
        }

        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            bootstrap.upload_templates_to_drive(&cfg),
        )
        .await
        {
            Ok(Ok(_)) => {
                trace!("Templates uploaded to drive successfully");
            }
            Ok(Err(e)) => {
                warn!("Template drive upload error (non-blocking): {}", e);
            }
            Err(_) => {
                warn!("Template drive upload timed out after 30s, continuing startup...");
            }
        }

        Ok::<AppConfig, std::io::Error>(cfg)
    };

    trace!("Bootstrap config phase complete");
    cfg
}
pub async fn init_database(
    progress_tx: &tokio::sync::mpsc::UnboundedSender<BootstrapProgress>,
) -> Result<botcore::shared::utils::DbPool, std::io::Error> {
    trace!("Creating database pool again...");
    progress_tx.send(BootstrapProgress::ConnectingDatabase).ok();

    // Get database URL from Vault
    if let Ok(secrets) = crate::core::secrets::SecretsManager::get() {
        if let Ok(db_url) = secrets.get_database_url().await {
            std::env::set_var("DATABASE_URL", &db_url);
            info!("Database URL obtained from Vault");
        } else {
            warn!("Failed to get database URL from Vault, trying DATABASE_URL env var");
        }
    } else {
        info!("SecretsManager not available, trying DATABASE_URL env var");
    }

    let pool = match create_conn() {
        Ok(pool) => {
            trace!("Running database migrations...");
            info!("Running database migrations...");
            if let Err(e) = {
    let mut conn = pool.get().map_err(|e| std::io::Error::other(format!("Pool error: {e}")))?;
    run_diesel_migrations(&mut conn)
    } {
                error!("Failed to run migrations: {}", e);

                warn!("Continuing despite migration errors - database might be partially migrated");
            } else {
                info!("Database migrations completed successfully");
            }

            #[cfg(feature = "sampledata")]
            {
                // PRODUCTION GUARD: demo data never runs in prod unless
                // explicitly enabled with GB_ALLOW_SAMPLEDATA=1.
                if std::env::var("GB_ALLOW_SAMPLEDATA").is_ok() {
                    botsampledata::seed_all(&pool);
                } else {
                    info!("botsampledata: DB demo seeding skipped (GB_ALLOW_SAMPLEDATA not set)");
                }
            }

            #[cfg(feature = "saas")]
            {
                if let Ok(mut conn) = pool.get() {
                    diesel::sql_query(
                        "INSERT INTO tenants (id, name, slug, is_active, created_at, updated_at) \
                         VALUES ('00000000-0000-0000-0000-000000000000', 'System', 'system', true, NOW(), NOW()) \
                         ON CONFLICT (id) DO NOTHING",
                    )
                    .execute(&mut conn)
                    .ok();
                    diesel::sql_query(
                        "INSERT INTO organizations (org_id, name, slug, domain, created_at, updated_at) \
                         VALUES ('00000000-0000-0000-0000-000000000000', 'System', 'system', NULL, NOW(), NOW()) \
                         ON CONFLICT (org_id) DO NOTHING",
                    )
                    .execute(&mut conn)
                    .ok();
                    diesel::sql_query(
                        "INSERT INTO branches (id, org_id, tenant_id, slug, name, is_active, created_at, updated_at) \
                         VALUES ('00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', 'system', 'System', true, NOW(), NOW()) \
                         ON CONFLICT (id) DO NOTHING",
                    )
                    .execute(&mut conn)
                    .ok();

                    // Default cloud catalog products are scoped to the dedicated
                    // sample tenant (not the nil/global scope) so the seeded
                    // products never pollute a real tenant.
                    #[cfg(feature = "sampledata")]
                    {
                        if std::env::var("GB_ALLOW_SAMPLEDATA").is_ok() {
                            botproducts::seed::seed_default_products(
                                &mut conn,
                                botsampledata::sample::SAMPLE_BRANCH_ID,
                            );
                        } else {
                            info!("botsampledata: product sample-catalog seeding skipped (GB_ALLOW_SAMPLEDATA not set)");
                        }
                    }
                    #[cfg(not(feature = "sampledata"))]
                    {
                        // Without the sampledata feature there is no dedicated
                        // sample tenant to scope the catalog to. Never degrade
                        // to the nil/global scope; skip product seeding instead.
                        warn!("Product seeding skipped: sample data feature is disabled");
                    }
                }
            }

            pool
        }
        Err(e) => {
            error!("Failed to create database pool: {}", e);
            progress_tx
                .send(BootstrapProgress::BootstrapError(format!(
                    "Database pool creation failed: {}",
                    e
                )))
                .ok();
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Database pool creation failed: {}", e),
            ));
        }
    };

    Ok(pool)
}
pub async fn load_config(
    pool: &botcore::shared::utils::DbPool,
) -> Result<AppConfig, std::io::Error> {
    info!("Loading config from database after template sync...");
    let refreshed_cfg = AppConfig::from_database(pool).unwrap_or_else(|e| {
        warn!(
            "Failed to load config from database: {}, falling back to env",
            e
        );
        AppConfig::from_env().unwrap_or_else(|e| {
            error!("Failed to load config from env: {}", e);
            AppConfig::default()
        })
    });
    let config = std::sync::Arc::new(refreshed_cfg.clone());
    info!(
        "Server configured to listen on {}:{}",
        config.server.host, config.server.port
    );

    Ok(refreshed_cfg)
}

/// Ensures the bot templates repository is cloned to `work/templates/`
/// and pulls latest changes (never rebase).
/// Falls back to default templates directory if git fails.
async fn ensure_templates_repo() {
    let repo_url = std::env::var("BOT_TEMPLATES_REPO")
        .unwrap_or_else(|_| "https://github.com/generalbots/templates.git".to_string());
    let work_dir = Path::new("work/templates");

    if work_dir.exists() {
        // git pull — never rebase
        let pull = Command::new("git")
            .args(["-C", "work/templates", "pull", "--ff-only"])
            .output();
        match pull {
            Ok(out) if out.status.success() => {
                info!("Templates repo updated via git pull");
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                warn!("git pull failed (non-blocking): {}", stderr);
            }
            Err(e) => {
                warn!("git pull could not run (non-blocking): {}", e);
            }
        }
    } else {
        // Clone fresh
        if let Some(parent) = work_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let clone = Command::new("git")
            .args(["clone", &repo_url, "work/templates"])
            .output();
        match clone {
            Ok(out) if out.status.success() => {
                info!("Templates repo cloned to work/templates");
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                warn!("git clone failed (non-blocking): {}", stderr);
                let _ = std::fs::remove_dir_all("work/templates");
            }
            Err(e) => {
                warn!("git clone could not run (non-blocking): {}", e);
            }
        }
    }
}
