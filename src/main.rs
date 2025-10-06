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

use log::info;
use qdrant_client::Qdrant;
use std::sync::Arc;

use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use sqlx::PgPool;

use crate::auth::AuthService;
use crate::bot::BotOrchestrator;
use crate::config::AppConfig;
use crate::email::{
    get_emails, get_latest_email_from, list_emails, save_click, save_draft, send_email,
};
use crate::file::{list_file, upload_file};
use crate::llm::llm_generic::generic_chat_completions;
use crate::llm::llm_local::{
    chat_completions_local, embeddings_local, ensure_llama_servers_running,
};
use crate::session::SessionManager;
use crate::shared::state::AppState;
use crate::tools::{RedisToolExecutor, ToolManager};
use crate::web_automation::{initialize_browser_pool, BrowserPool};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting General Bots 6.0...");

    let config = AppConfig::from_env();
    let db_url = config.database_url();
    let db_custom_url = config.database_custom_url();
    let db = PgPool::connect(&db_url).await.unwrap();
    let db_custom = PgPool::connect(&db_custom_url).await.unwrap();

    let minio_client = init_minio(&config)
        .await
        .expect("Failed to initialize Minio");

    let browser_pool = Arc::new(BrowserPool::new(
        "http://localhost:9515".to_string(),
        5,
        "/usr/bin/brave-browser-beta".to_string(),
    ));

    ensure_llama_servers_running()
        .await
        .expect("Failed to initialize LLM local server.");

    initialize_browser_pool()
        .await
        .expect("Failed to initialize browser pool");

    // Initialize Redis if available
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "".to_string());
    let redis_conn = match std::env::var("REDIS_URL") {
        Ok(redis_url_value) => {
            let client = redis::Client::open(redis_url_value.clone())
                .expect("Failed to create Redis client");
            let conn = client
                .get_connection()
                .expect("Failed to create Redis connection");
            Some(Arc::new(conn))
        }
        Err(_) => None,
    };

    let qdrant_url = std::env::var("QDRANT_URL").unwrap_or("http://localhost:6334".to_string());
    let qdrant = Qdrant::from_url(&qdrant_url)
        .build()
        .expect("Failed to connect to Qdrant");

    let session_manager = SessionManager::new(db.clone(), redis_conn.clone());
    let auth_service = AuthService::new(db.clone(), redis_conn.clone());

    let llm_provider: Arc<dyn crate::llm::LLMProvider> = match std::env::var("LLM_PROVIDER")
        .unwrap_or("mock".to_string())
        .as_str()
    {
        "openai" => Arc::new(crate::llm::OpenAIClient::new(
            std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY required"),
        )),
        "anthropic" => Arc::new(crate::llm::AnthropicClient::new(
            std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY required"),
        )),
        _ => Arc::new(crate::llm::MockLLMProvider::new()),
    };

    let web_adapter = Arc::new(crate::channels::WebChannelAdapter::new());
    let voice_adapter = Arc::new(crate::channels::VoiceAdapter::new(
        std::env::var("LIVEKIT_URL").unwrap_or("ws://localhost:7880".to_string()),
        std::env::var("LIVEKIT_API_KEY").unwrap_or("dev".to_string()),
        std::env::var("LIVEKIT_API_SECRET").unwrap_or("secret".to_string()),
    ));

    let whatsapp_adapter = Arc::new(crate::whatsapp::WhatsAppAdapter::new(
        std::env::var("META_ACCESS_TOKEN").unwrap_or("".to_string()),
        std::env::var("META_PHONE_NUMBER_ID").unwrap_or("".to_string()),
        std::env::var("META_WEBHOOK_VERIFY_TOKEN").unwrap_or("".to_string()),
    ));

    let tool_executor = Arc::new(
        RedisToolExecutor::new(
            redis_url.as_str(),
            web_adapter.clone() as Arc<dyn crate::channels::ChannelAdapter>,
            db.clone(),
            redis_conn.clone(),
        )
        .expect("Failed to create RedisToolExecutor"),
    );
    let chart_generator = ChartGenerator::new().map(Arc::new);
    // Initialize LangChain components
    let llm = OpenAI::default();
    let llm_provider: Arc<dyn LLMProvider> = Arc::new(OpenAIClient::new(llm));

    // Initialize vector store for document mode
    let vector_store = if let (Ok(qdrant_url), Ok(openai_key)) =
        (std::env::var("QDRANT_URL"), std::env::var("OPENAI_API_KEY"))
    {
        let embedder = OpenAiEmbedder::default().with_api_key(openai_key);
        let client = QdrantClient::from_url(&qdrant_url).build().ok()?;

        let store = StoreBuilder::new()
            .embedder(embedder)
            .client(client)
            .collection_name("documents")
            .build()
            .await
            .ok()?;

        Some(Arc::new(store))
    } else {
        None
    };

    // Initialize SQL chain for database mode
    let sql_chain = if let Ok(db_url) = std::env::var("DATABASE_URL") {
        let engine = PostgreSQLEngine::new(&db_url).await.ok()?;
        let db = SQLDatabaseBuilder::new(engine).build().await.ok()?;

        let llm = OpenAI::default();
        let chain = langchain_rust::chain::SQLDatabaseChainBuilder::new()
            .llm(llm)
            .top_k(5)
            .database(db)
            .build()
            .ok()?;

        Some(Arc::new(chain))
    } else {
        None
    };

    let tool_manager = ToolManager::new();
    let orchestrator = BotOrchestrator::new(
        session_manager,
        tool_manager,
        llm_provider,
        auth_service,
        chart_generator,
        vector_store,
        sql_chain,
    );

    orchestrator.add_channel("web", web_adapter.clone());
    orchestrator.add_channel("voice", voice_adapter.clone());
    orchestrator.add_channel("whatsapp", whatsapp_adapter.clone());

    sqlx::query(
        "INSERT INTO bots (id, name, llm_provider) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap())
    .bind("Default Bot")
    .bind("mock")
    .execute(&db)
    .await
    .unwrap();

    let app_state = web::Data::new(AppState {
        db: db.into(),
        db_custom: db_custom.into(),
        config: Some(config.clone()),
        minio_client: minio_client.into(),
        browser_pool: browser_pool.clone(),
        orchestrator: Arc::new(orchestrator),
        web_adapter,
        voice_adapter,
        whatsapp_adapter,
    });

    // Start automation service in background
    let automation_state = app_state.get_ref().clone();

    let automation = AutomationService::new(automation_state, "src/prompts");
    let _automation_handle = automation.spawn();

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Logger::new("HTTP REQUEST: %a %{User-Agent}i"))
            .app_data(app_state.clone())
            // Original services
            .service(upload_file)
            .service(list_file)
            .service(save_click)
            .service(get_emails)
            .service(list_emails)
            .service(send_email)
            .service(crate::orchestrator::chat_stream)
            .service(crate::orchestrator::chat)
            .service(chat_completions_local)
            .service(save_draft)
            .service(generic_chat_completions)
            .service(embeddings_local)
            .service(get_latest_email_from)
            .service(services::orchestrator::websocket_handler)
            .service(services::orchestrator::whatsapp_webhook_verify)
            .service(services::orchestrator::whatsapp_webhook)
            .service(services::orchestrator::voice_start)
            .service(services::orchestrator::voice_stop)
            .service(services::orchestrator::create_session)
            .service(services::orchestrator::get_sessions)
            .service(services::orchestrator::get_session_history)
            .service(services::orchestrator::index)
            .service(create_organization)
            .service(get_organization)
            .service(list_organizations)
            .service(update_organization)
            .service(delete_organization)
    })
    .bind((config.server.host.clone(), config.server.port))?
    .run()
    .await
}
