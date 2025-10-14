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
mod bot;
mod channels;
mod config;
mod context;
#[cfg(feature = "email")]
mod email;
mod file;
mod llm;
mod llm_legacy;
mod org;
mod session;
mod shared;
mod tools;
mod whatsapp;
use crate::bot::{
    auth_handler, create_session, get_session_history, get_sessions, index, set_mode_handler,
    start_session, static_files, voice_start, voice_stop, websocket_handler,
    whatsapp_webhook_verify,
};
use crate::channels::{VoiceAdapter, WebChannelAdapter};
use crate::config::AppConfig;
#[cfg(feature = "email")]
use crate::email::{
    get_emails, get_latest_email_from, list_emails, save_click, save_draft, send_email,
};
use crate::file::upload_file;
use crate::llm_legacy::llm_local::{
    chat_completions_local, embeddings_local, ensure_llama_servers_running,
};
use crate::shared::state::AppState;
use crate::whatsapp::WhatsAppAdapter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cfg = AppConfig::from_env();
    let config = std::sync::Arc::new(cfg.clone());

    let db_pool = match diesel::Connection::establish(&cfg.database_url()) {
        Ok(conn) => {
            info!("Connected to main database successfully");
            Arc::new(Mutex::new(conn))
        }
        Err(e) => {
            log::error!("Failed to connect to main database: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Database connection failed: {}", e),
            ));
        }
    };

    let custom_db_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        cfg.database_custom.username,
        cfg.database_custom.password,
        cfg.database_custom.server,
        cfg.database_custom.port,
        cfg.database_custom.database
    );

    let db_custom_pool = match diesel::Connection::establish(&custom_db_url) {
        Ok(conn) => {
            info!("Connected to custom database successfully");
            Arc::new(Mutex::new(conn))
        }
        Err(e2) => {
            log::error!("Failed to connect to custom database: {}", e2);
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Custom Database connection failed: {}", e2),
            ));
        }
    };

    ensure_llama_servers_running()
        .await
        .expect("Failed to initialize LLM local server.");

    let redis_client = match redis::Client::open("redis://127.0.0.1/") {
        Ok(client) => {
            info!("Connected to Redis successfully");
            Some(Arc::new(client))
        }
        Err(e) => {
            log::warn!("Failed to connect to Redis: {}", e);
            None
        }
    };

    let tool_manager = Arc::new(tools::ToolManager::new());
    let llama_url =
        std::env::var("LLM_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let llm_provider = Arc::new(crate::llm::OpenAIClient::new(
        "empty".to_string(),
        Some(llama_url.clone()),
    ));

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

    let session_manager = Arc::new(tokio::sync::Mutex::new(session::SessionManager::new(
        diesel::Connection::establish(&cfg.database_url()).unwrap(),
        redis_client.clone(),
    )));

    let auth_service = Arc::new(tokio::sync::Mutex::new(auth::AuthService::new(
        diesel::Connection::establish(&cfg.database_url()).unwrap(),
        redis_client.clone(),
    )));

    let app_state = Arc::new(AppState {
        s3_client: None,
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
            map.insert(
                "web".to_string(),
                web_adapter.clone() as Arc<dyn crate::channels::ChannelAdapter>,
            );
            map
        })),
        response_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        web_adapter: web_adapter.clone(),
        voice_adapter: voice_adapter.clone(),
        whatsapp_adapter: whatsapp_adapter.clone(),
        tool_api: tool_api.clone(),
    });

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
            .service(set_mode_handler)
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
    .bind((config.server.host.clone(), config.server.port))?
    .run()
    .await
}
