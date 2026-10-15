//! state_builder - extracted from bootstrap.rs

use crate::core::bot::channels::{VoiceAdapter, WebChannelAdapter};
use crate::core::bot_database::BotDatabaseManager;
use crate::core::config::AppConfig;
use crate::drive::s3_repository::S3Repository;
use botcore::config::ConfigManager;
use botcore::shared::state::AppState;
use log::{error, info, warn};
use std::sync::Arc;

use super::directory_setup::{bootstrap_directory_admin, init_directory_service};
use super::llm_setup::init_llm_provider;

pub async fn create_app_state(
    cfg: AppConfig,
    pool: botcore::shared::utils::DbPool,
    #[cfg(feature = "cache")] redis_client: &Option<Arc<redis::Client>>,
) -> Result<Arc<AppState>, std::io::Error> {
    use std::collections::HashMap;

    #[cfg(feature = "cache")]
    let redis_client = redis_client.clone();
    #[cfg(not(feature = "cache"))]
    let redis_client: Option<Arc<redis::Client>> = None;

    let web_adapter = Arc::new(WebChannelAdapter::new());
    let voice_adapter = Arc::new(VoiceAdapter::new());

    #[cfg(feature = "drive")]
    let drive_initialized = if cfg.drive.is_valid() {
        match S3Repository::new(
            &cfg.drive.endpoint,
            &cfg.drive.access_key,
            &cfg.drive.secret_key,
            &cfg.drive.bucket,
        ) {
            Ok(client) => {
                if let Err(e) = client.create_bucket_if_not_exists("default.gborg").await {
                    warn!("Failed to create default.gborg bucket: {}", e);
                }
                super::ensure_vendor_files_in_minio(&client).await;
                Some(std::sync::Arc::new(client)
                    as std::sync::Arc<dyn botlib::traits::DriveRepository>)
            }
            Err(e) => {
                warn!("Failed to initialize S3 client: {}", e);
                None
            }
        }
    } else {
        info!("Drive credentials not configured — skipping MinIO/Drive initialization");
        None
    };

    let session_manager_inner =
        crate::core::session::LocalSessionManager(botcoresession::SessionManager::new(
            pool.clone(),
            #[cfg(feature = "cache")]
            redis_client.clone(),
        ));
    let session_manager: Arc<tokio::sync::Mutex<dyn botlib::traits::SessionManagerService>> =
        Arc::new(tokio::sync::Mutex::new(session_manager_inner));

    #[cfg(feature = "directory")]
    let (auth_service, zitadel_config) = init_directory_service().await?;

    #[cfg(feature = "directory")]
    {
        let skip_local_directory = cfg!(windows)
            && std::env::var("GBO_SKIP_LOCAL_DIRECTORY")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
        if skip_local_directory {
            info!("Skipping local directory bootstrap for this Windows development run");
        } else {
            bootstrap_directory_admin(&zitadel_config).await;
        }
    }

    let config_manager = ConfigManager::new(pool.clone());

    let _ = pool
        .get()
        .map_err(|e| std::io::Error::other(format!("Failed to get database connection: {}", e)))?;
    let (default_bot_id_str, default_bot_name) = crate::core::bot::get_default_bot();
    let default_bot_id = uuid::Uuid::parse_str(&default_bot_id_str).unwrap_or_default();
    info!(
        "Using default bot: {} (id: {})",
        default_bot_name, default_bot_id
    );

    // Check environment variables first for LLM configuration
    let llm_url_env = std::env::var("LLM_URL").ok();
    let llm_url = if let Some(url) = llm_url_env {
        info!("Using LLM URL from environment variable: {}", url);
        url
    } else {
        config_manager
            .get_config(&default_bot_id, "llm-url", Some(""))
            .unwrap_or_else(|_| "".to_string())
    };
    info!("LLM URL: {}", llm_url);

    let llm_model_env = std::env::var("LLM_MODEL").ok();
    let llm_model = if let Some(model) = llm_model_env {
        info!("Using LLM model from environment variable: {}", model);
        model
    } else {
        config_manager
            .get_config(&default_bot_id, "llm-model", Some(""))
            .unwrap_or_default()
    };
    if !llm_model.is_empty() {
        info!("LLM Model: {}", llm_model);
    }

    let llm_key = std::env::var("LLM_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .or_else(|_| {
            config_manager
                .get_config(&default_bot_id, "llm-key", Some(""))
                .map_err(|_| std::env::VarError::NotPresent)
        })
        .unwrap_or_default();

    // If llm-url points to external API but no key is configured, fall back to local LLM
    let llm_url = if llm_key.is_empty()
        && !llm_url.contains("localhost")
        && !llm_url.contains("127.0.0.1")
        && (llm_url.contains("api.z.ai")
            || llm_url.contains("openai.com")
            || llm_url.contains("anthropic.com"))
    {
        warn!("External LLM URL configured ({}), but no API key provided. Falling back to local LLM at ", llm_url);
        "".to_string()
    } else {
        llm_url
    };

    // LLM endpoint path configuration. Default to empty so
    // create_llm_provider_from_url derives the correct path from the URL —
    // forcing /v1/chat/completions appended to a full chat-completions URL
    // (e.g. Kilo's https://api.kilo.ai/api/gateway/chat/completions) produced
    // "Invalid path" 400s for every boot-time provider user (AutoTask).
    let llm_endpoint_path = config_manager
        .get_config(&default_bot_id, "llm-endpoint-path", Some(""))
        .unwrap_or_default();

    #[cfg(feature = "llm")]
    let base_llm_provider = crate::llm::provider_factory::create_llm_provider_from_url(
        &llm_url,
        if llm_model.is_empty() {
            None
        } else {
            Some(llm_model.clone())
        },
        Some(llm_endpoint_path.clone()),
        None,
    );

    #[cfg(feature = "llm")]
    let dynamic_llm_provider = Arc::new(crate::llm::dynamic_provider::DynamicLLMProvider::new(base_llm_provider));

    #[cfg(feature = "llm")]
    {
        // Ensure the DynamicLLMProvider is initialized with the correct config from database
        // This makes the system robust: even if the URL was set before server startup,
        // the provider will use the correct configuration
        info!(
            "Initializing DynamicLLMProvider with config: URL={}, Model={}, Endpoint={}",
            llm_url,
            if llm_model.is_empty() {
                "(default)"
            } else {
                &llm_model
            },
            llm_endpoint_path.clone()
        );
        #[cfg(feature = "llm")]
        dynamic_llm_provider
            .update_from_config(
                &llm_url,
                if llm_model.is_empty() {
                    None
                } else {
                    Some(llm_model.clone())
                },
                Some(llm_endpoint_path),
                None,
            )
            .await;
        info!("DynamicLLMProvider initialized successfully");
    }

    #[cfg(feature = "llm")]
    let llm_provider = init_llm_provider(
        &config_manager,
        &default_bot_id.to_string(),
        dynamic_llm_provider.clone(),
        &pool,
        redis_client.clone(),
    );

    #[cfg(any(feature = "research", feature = "llm"))]
    let kb_manager = Arc::new(crate::core::kb::KnowledgeBaseManager::with_bot_config(
        "work",
        pool.clone(),
        default_bot_id,
    ));

    let metrics_collector = botcore::shared::analytics::MetricsCollector::new();

    #[cfg(feature = "monitoring")]
    let tracing_service = botmonitoring::DistributedTracingService::new();

    #[cfg(feature = "tasks")]
    let task_scheduler = None;

    let (attendant_tx, _attendant_rx) =
        tokio::sync::broadcast::channel::<botcore::shared::state::AttendantNotification>(1000);

    let (task_progress_tx, _task_progress_rx) =
        tokio::sync::broadcast::channel::<botcore::shared::state::TaskProgressEvent>(1000);

    // Initialize BotDatabaseManager for per-bot database support
    let database_url = botcore::shared::utils::get_database_url_sync().unwrap_or_default();
    let bot_database_manager = Arc::new(BotDatabaseManager::new(pool.clone(), &database_url));

    // Sync all bot databases on startup - ensures each bot has its own database
    info!("Syncing bot databases on startup...");
    match bot_database_manager.sync_all_bot_databases() {
        Ok(sync_result) => {
            info!(
                "Bot database sync complete: {} created, {} verified, {} errors",
                sync_result.databases_created,
                sync_result.databases_verified,
                sync_result.errors.len()
            );
            for err in &sync_result.errors {
                warn!("Bot database sync error: {}", err);
            }
        }
        Err(e) => {
            error!("Failed to sync bot databases: {}", e);
        }
    }

    botcorepkg::plugin::init_global_registry();

    let app_state = Arc::new(AppState {
        #[cfg(feature = "drive")]
        drive: drive_initialized,
        #[cfg(not(feature = "drive"))]
        drive: None,
        config: Some(cfg.clone()),
        conn: pool.clone(),
        database_url: database_url.clone(),
        bot_database_manager: bot_database_manager.clone(),
        bucket_name: cfg.drive.bucket.clone(),
        #[cfg(feature = "cache")]
        cache: redis_client.clone(),
        session_manager,
        metrics_collector,
        #[cfg(feature = "tasks")]
        task_scheduler,
        #[cfg(not(feature = "tasks"))]
        task_scheduler: None,
        #[cfg(feature = "llm")]
        llm_provider: Some(Arc::new(crate::llm::BotlibLLMProviderWrapper::new(
            llm_provider.clone(),
            String::new(),
            String::new(),
        )) as Arc<dyn botlib::traits::LLMProvider>),
        #[cfg(feature = "llm")]
        dynamic_llm_provider: Some(dynamic_llm_provider.clone()),
        #[cfg(feature = "directory")]
        auth_service: Some(
            auth_service.clone() as Arc<tokio::sync::Mutex<dyn botlib::traits::AuthServiceTrait>>
        ),
        channels: Arc::new(tokio::sync::Mutex::new({
            let mut map = HashMap::new();
            map.insert(
                "web".to_string(),
                web_adapter.clone() as Arc<dyn crate::core::bot::channels::ChannelAdapter>,
            );
            map
        })),
        response_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        active_streams: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        hear_channels: Arc::new(std::sync::Mutex::new(HashMap::new())),
        web_adapter: web_adapter.clone(),
        voice_adapter: voice_adapter.clone(),
        #[cfg(any(feature = "research", feature = "llm"))]
        kb_manager: Some(kb_manager.clone() as std::sync::Arc<dyn botlib::traits::KnowledgeBase>),
        #[cfg(not(any(feature = "research", feature = "llm")))]
        kb_manager: None,
        task_engine: None,
        extensions: {
            let ext = botcore::shared::state::Extensions::new();
            #[cfg(feature = "llm")]
            ext.insert_blocking(Arc::clone(&dynamic_llm_provider));
            #[cfg(feature = "monitoring")]
            ext.insert_blocking(tracing_service.clone());
            ext
        },
        attendant_broadcast: Some(attendant_tx),
        task_progress_broadcast: Some(task_progress_tx),
        billing_alert_broadcast: None,
        task_manifests: Arc::new(std::sync::RwLock::new(HashMap::new())),
        #[cfg(feature = "terminal")]
        terminal_manager: Some(Arc::new(crate::api::terminal::TerminalManager::new())
            as botcore::shared::state::UnresolvedService),
        #[cfg(not(feature = "terminal"))]
        terminal_manager: None,
        #[cfg(feature = "project")]
        project_service: Some(Arc::new(tokio::sync::RwLock::new(
            crate::project::ProjectService::new(),
        )) as botcore::shared::state::UnresolvedService),
        #[cfg(not(feature = "project"))]
        project_service: None,
        #[cfg(feature = "compliance")]
        legal_service: Some(
            Arc::new(tokio::sync::RwLock::new(crate::legal::LegalService::new()))
                as botcore::shared::state::UnresolvedService,
        ),
        #[cfg(not(feature = "compliance"))]
        legal_service: None,
        jwt_manager: None,
        auth_provider_registry: None,
        rbac_manager: None,
        start_bas_guards: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        pending_stream_responses: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        script_runner: None,
    });

    Ok(app_state)
}
