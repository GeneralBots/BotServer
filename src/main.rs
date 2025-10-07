use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use log::info;
use std::sync::Arc;

mod auth;
mod automation;
mod basic;
mod bot;
mod channels;
mod chart;
mod config;
mod context;
mod email;
mod file;
mod llm;
mod llm_legacy;
mod org;
mod session;
mod shared;
mod tools;
mod web_automation;
mod whatsapp;

use crate::automation::AutomationService;
use crate::bot::{
    create_session, get_session_history, get_sessions, index, set_mode_handler, static_files,
    voice_start, voice_stop, websocket_handler, whatsapp_webhook, whatsapp_webhook_verify,
};
use crate::channels::{VoiceAdapter, WebChannelAdapter};
use crate::config::AppConfig;
use crate::email::{
    get_emails, get_latest_email_from, list_emails, save_click, save_draft, send_email,
};
use crate::file::{list_file, upload_file};
use crate::llm_legacy::llm_generic::generic_chat_completions;
use crate::llm_legacy::llm_local::{
    chat_completions_local, embeddings_local, ensure_llama_servers_running,
};
use crate::shared::AppState;
use crate::whatsapp::WhatsAppAdapter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting General Bots 6.0...");

    let config = AppConfig::from_env();

    // Main database pool (required)
    let db_pool = match sqlx::postgres::PgPool::connect(&config.database_url()).await {
        Ok(pool) => {
            info!("Connected to main database");
            pool
        }
        Err(e) => {
            log::error!("Failed to connect to main database: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Database connection failed: {}", e),
            ));
        }
    };

    // Optional custom database pool
    let db_custom_pool = match sqlx::postgres::PgPool::connect(&config.database_custom_url()).await
    {
        Ok(pool) => {
            info!("Connected to custom database");
            Some(pool)
        }
        Err(e) => {
            log::warn!("Failed to connect to custom database: {}", e);
            None
        }
    };

    // Optional Redis client
    let redis_client = match redis::Client::open("redis://127.0.0.1/") {
        Ok(client) => {
            info!("Connected to Redis");
            Some(Arc::new(client))
        }
        Err(e) => {
            log::warn!("Failed to connect to Redis: {}", e);
            None
        }
    };

    // Initialize MinIO client
    let minio_client = file::init_minio(&config)
        .await
        .expect("Failed to initialize Minio");

    // Initialize browser pool
    let browser_pool = Arc::new(web_automation::BrowserPool::new(
        "chrome".to_string(),
        2,
        "headless".to_string(),
    ));

    // Initialize LLM servers
    ensure_llama_servers_running()
        .await
        .expect("Failed to initialize LLM local server.");

    web_automation::initialize_browser_pool()
        .await
        .expect("Failed to initialize browser pool");

    // Initialize services from new architecture
    let auth_service = auth::AuthService::new(db_pool.clone(), redis_client.clone());
    let session_manager = session::SessionManager::new(db_pool.clone(), redis_client.clone());

    let tool_manager = tools::ToolManager::new();
    let llm_provider = Arc::new(llm::MockLLMProvider::new());

    let orchestrator =
        bot::BotOrchestrator::new(session_manager, tool_manager, llm_provider, auth_service);

    let web_adapter = Arc::new(WebChannelAdapter::new());
    let voice_adapter = Arc::new(VoiceAdapter::new(
        "https://livekit.example.com".to_string(),
        "api_key".to_string(),
        "api_secret".to_string(),
    ));

    let whatsapp_adapter = Arc::new(WhatsAppAdapter::new(
        "whatsapp_token".to_string(),
        "phone_number_id".to_string(),
        "verify_token".to_string(),
    ));

    let tool_api = Arc::new(tools::ToolApi::new());

    // Create unified app state
    let app_state = AppState {
        minio_client: Some(minio_client),
        config: Some(config.clone()),
        db: Some(db_pool.clone()),
        db_custom: db_custom_pool.clone(),
        browser_pool: browser_pool.clone(),
        orchestrator: Arc::new(orchestrator),
        web_adapter,
        voice_adapter,
        whatsapp_adapter,
        tool_api,
    };

    // Start automation service in background
    let automation_state = app_state.clone();
    let automation = AutomationService::new(automation_state, "src/prompts");
    let _automation_handle = automation.spawn();

    info!(
        "Starting server on {}:{}",
        config.server.host, config.server.port
    );

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(Logger::new("HTTP REQUEST: %a %{User-Agent}i"))
            .app_data(web::Data::new(app_state.clone()))
            // Legacy services
            .service(upload_file)
            .service(list_file)
            .service(save_click)
            .service(get_emails)
            .service(list_emails)
            .service(send_email)
            .service(chat_completions_local)
            .service(save_draft)
            .service(generic_chat_completions)
            .service(embeddings_local)
            .service(get_latest_email_from)
            // New bot services
            .service(index)
            .service(static_files)
            .service(websocket_handler)
            .service(whatsapp_webhook_verify)
            .service(whatsapp_webhook)
            .service(voice_start)
            .service(voice_stop)
            .service(create_session)
            .service(get_sessions)
            .service(get_session_history)
            .service(set_mode_handler)
    })
    .bind((config.server.host.clone(), config.server.port))?
    .run()
    .await
}
