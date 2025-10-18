#![allow(dead_code)]
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use dotenvy::dotenv;
use log::info;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod auth;
mod automation;
mod basic;
mod bootstrap;
mod bot;
mod channels;
mod config;
mod context;
mod drive_monitor;
#[cfg(feature = "email")]
mod email;
mod file;
mod kb;
mod llm;
mod llm_legacy;
mod meet;
mod org;
mod package_manager;
mod session;
mod shared;
mod tools;
#[cfg(feature = "web_automation")]
mod web_automation;
mod web_server;
mod whatsapp;

use crate::auth::auth_handler;
use crate::automation::AutomationService;
use crate::bot::{start_session, websocket_handler};
use crate::bootstrap::BootstrapManager;
use crate::channels::{VoiceAdapter, WebChannelAdapter};
use crate::config::AppConfig;
use crate::drive_monitor::DriveMonitor;
#[cfg(feature = "email")]
use crate::email::{get_emails, get_latest_email_from, list_emails, save_click, save_draft, send_email};
use crate::file::{init_drive, upload_file};
use crate::llm_legacy::llm_local::{chat_completions_local, embeddings_local, ensure_llama_servers_running};
use crate::meet::{voice_start, voice_stop};
use crate::package_manager::InstallMode;
use crate::session::{create_session, get_session_history, get_sessions};
use crate::shared::state::AppState;
use crate::web_server::{index, static_files};
use crate::whatsapp::whatsapp_webhook_verify;
use crate::whatsapp::WhatsAppAdapter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 {
        let command = &args[1];
        match command.as_str() {
            "install" | "remove" | "list" | "status" | "--help" | "-h" => {
                match package_manager::cli::run() {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        eprintln!("CLI error: {}", e);
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("CLI command failed: {}", e)));
                    }
                }
            }
            _ => {
                eprintln!("Unknown command: {}", command);
                eprintln!("Run 'botserver --help' for usage information");
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Unknown command: {}", command)));
            }
        }
    }

    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting BotServer bootstrap process");
    
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

    let mut bootstrap = BootstrapManager::new(install_mode.clone(), tenant.clone());
    let cfg = match bootstrap.bootstrap() {
        Ok(config) => {
            info!("Bootstrap completed successfully, configuration loaded from database");
            config
        }
        Err(e) => {
            log::error!("Bootstrap failed: {}", e);
            info!("Attempting to load configuration from database");
            match diesel::Connection::establish(&format!("postgres://localhost:5432/botserver_db")) {
                Ok(mut conn) => AppConfig::from_database(&mut conn),
                Err(_) => {
                    info!("Database not available, using environment variables as fallback");
                    AppConfig::from_env()
                }
            }
        }
    };

    let config = std::sync::Arc::new(cfg.clone());
    
    info!("Establishing database connection to {}", cfg.database_url());
    let db_pool = match diesel::Connection::establish(&cfg.database_url()) {
        Ok(conn) => Arc::new(Mutex::new(conn)),
        Err(e) => {
            log::error!("Failed to connect to main database: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, format!("Database connection failed: {}", e)));
        }
    };

    let db_custom_pool = db_pool.clone();

    info!("Initializing LLM server at {}", cfg.ai.endpoint);
    ensure_llama_servers_running().await.expect("Failed to initialize LLM local server");

    let cache_url = cfg.config_path("cache").join("redis.conf").display().to_string();
    let redis_client = match redis::Client::open(cache_url.as_str()) {
        Ok(client) => Some(Arc::new(client)),
        Err(e) => {
            log::warn!("Failed to connect to Redis: {}", e);
            None
        }
    };

    let tool_manager = Arc::new(tools::ToolManager::new());
    let llm_provider = Arc::new(crate::llm::OpenAIClient::new("empty".to_string(), Some(cfg.ai.endpoint.clone())));
    
    let web_adapter = Arc::new(WebChannelAdapter::new());
    let voice_adapter = Arc::new(VoiceAdapter::new("https://livekit.example.com".to_string(), "api_key".to_string(), "api_secret".to_string()));
    let whatsapp_adapter = Arc::new(WhatsAppAdapter::new("whatsapp_token".to_string(), "phone_number_id".to_string(), "verify_token".to_string()));
    let tool_api = Arc::new(tools::ToolApi::new());

    info!("Initializing MinIO drive at {}", cfg.minio.server);
    let drive = init_drive(&config.minio).await.expect("Failed to initialize Drive");

    let session_manager = Arc::new(tokio::sync::Mutex::new(session::SessionManager::new(diesel::Connection::establish(&cfg.database_url()).unwrap(), redis_client.clone())));
    let auth_service = Arc::new(tokio::sync::Mutex::new(auth::AuthService::new(diesel::Connection::establish(&cfg.database_url()).unwrap(), redis_client.clone())));

    let app_state = Arc::new(AppState {
        s3_client: Some(drive.clone()),
        config: Some(cfg.clone()),
        conn: db_pool.clone(),
        custom_conn: db_custom_pool.clone(),
        redis_client: redis_client.clone(),
        session_manager: session_manager.clone(),
        tool_manager: tool_manager.clone(),
        llm_provider: llm_provider.clone(),
        auth_service: auth_service.clone(),
        channels: Arc::new(Mutex::new({
            let mut map = HashMap::new();
            map.insert("web".to_string(), web_adapter.clone() as Arc<dyn crate::channels::ChannelAdapter>);
            map
        })),
        response_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        web_adapter: web_adapter.clone(),
        voice_adapter: voice_adapter.clone(),
        whatsapp_adapter: whatsapp_adapter.clone(),
        tool_api: tool_api.clone(),
    });

    info!("Starting HTTP server on {}:{}", config.server.host, config.server.port);

    let worker_count = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

    let automation_state = app_state.clone();
    let automation = AutomationService::new(automation_state, "templates/announcements.gbai/announcements.gbdialog");
    let _automation_handle = automation.spawn();

    let drive_state = app_state.clone();
    let bucket_name = format!("{}default.gbai", cfg.minio.org_prefix);
    let drive_monitor = Arc::new(DriveMonitor::new(drive_state, bucket_name));
    let _drive_handle = drive_monitor.spawn();

    HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin().allow_any_method().allow_any_header().max_age(3600);
        let app_state_clone = app_state.clone();
        
        let mut app = App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(Logger::new("HTTP REQUEST: %a %{User-Agent}i"))
            .app_data(web::Data::from(app_state_clone));

        app = app
            .service(upload_file)
            .service(index)
            .service(static_files)
            .service(websocket_handler)
            .service(auth_handler)
            .service(whatsapp_webhook_verify)
            .service(voice_start)
            .service(voice_stop)
            .service(create_session)
            .service(get_sessions)
            .service(start_session)
            .service(get_session_history)
            .service(chat_completions_local)
            .service(embeddings_local);

        #[cfg(feature = "email")]
        {
            app = app
                .service(get_latest_email_from)
                .service(get_emails)
                .service(list_emails)
                .service(send_email)
                .service(save_draft)
                .service(save_click);
        }

        app
    })
    .workers(worker_count)
    .bind((config.server.host.clone(), config.server.port))?
    .run()
    .await
}
