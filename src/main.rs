use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
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
mod org;
mod session;
mod shared;
mod tools;
mod web_automation;
mod whatsapp;

use crate::{config::AppConfig, shared::state::AppState};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    info!("🚀 Starting Bot Server...");

    let config = AppConfig::from_env();

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url())
        .await
        .expect("Failed to create database pool");

    let db_custom_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_custom_url())
        .await
        .expect("Failed to create custom database pool");

    let redis_client = redis::Client::open("redis://127.0.0.1/").ok();

    let auth_service = auth::AuthService::new(db_pool.clone(), redis_client.clone().map(Arc::new));
    let session_manager =
        session::SessionManager::new(db_pool.clone(), redis_client.clone().map(Arc::new));

    let tool_manager = tools::ToolManager::new();
    let tool_api = Arc::new(tools::ToolApi::new());

    let web_adapter = Arc::new(channels::WebChannelAdapter::new());
    let voice_adapter = Arc::new(channels::VoiceAdapter::new(
        "https://livekit.example.com".to_string(),
        "api_key".to_string(),
        "api_secret".to_string(),
    ));

    let whatsapp_adapter = Arc::new(whatsapp::WhatsAppAdapter::new(
        "whatsapp_token".to_string(),
        "phone_number_id".to_string(),
        "verify_token".to_string(),
    ));

    let llm_provider = Arc::new(llm::MockLLMProvider::new());

    let orchestrator = Arc::new(bot::BotOrchestrator::new(
        session_manager,
        tool_manager,
        llm_provider,
        auth_service,
    ));

    let browser_pool = Arc::new(web_automation::BrowserPool::new());

    let app_state = AppState {
        minio_client: None,
        config: Some(config),
        db: Some(db_pool),
        db_custom: Some(db_custom_pool),
        browser_pool,
        orchestrator,
        web_adapter,
        voice_adapter,
        whatsapp_adapter,
        tool_api,
    };

    info!("🌐 Server running on {}:{}", "127.0.0.1", 8080);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .service(bot::websocket_handler)
            .service(bot::whatsapp_webhook_verify)
            .service(bot::whatsapp_webhook)
            .service(bot::voice_start)
            .service(bot::voice_stop)
            .service(bot::create_session)
            .service(bot::get_sessions)
            .service(bot::get_session_history)
            .service(bot::set_mode_handler)
            .service(bot::index)
            .service(bot::static_files)
            .service(llm::chat_completions_local)
            .service(llm::embeddings_local)
            .service(llm::generic_chat_completions)
            .service(llm::health)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
