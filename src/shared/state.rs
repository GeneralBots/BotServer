use diesel::PgConnection;
use redis::Client;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::auth::AuthService;
use crate::bot::BotOrchestrator;
use crate::channels::{VoiceAdapter, WebChannelAdapter};
use crate::config::AppConfig;
use crate::llm::LLMProvider;
use crate::session::SessionManager;
use crate::tools::ToolApi;
use crate::web_automation::BrowserPool;
use crate::whatsapp::WhatsAppAdapter;

pub struct AppState {
    pub s3_client: Option<aws_sdk_s3::Client>,
    pub config: Option<AppConfig>,
    pub conn: Arc<Mutex<PgConnection>>,
    pub redis_client: Option<Arc<Client>>,
    pub browser_pool: Arc<BrowserPool>,
    pub orchestrator: Arc<BotOrchestrator>,
    pub web_adapter: Arc<WebChannelAdapter>,
    pub voice_adapter: Arc<VoiceAdapter>,
    pub whatsapp_adapter: Arc<WhatsAppAdapter>,
    pub tool_api: Arc<ToolApi>,
}

impl Default for AppState {
    fn default() -> Self {
        let conn = diesel::PgConnection::establish("postgres://user:pass@localhost:5432/db")
            .expect("Failed to connect to database");

        let session_manager = SessionManager::new(conn, None);
        let tool_manager = crate::tools::ToolManager::new();
        let llm_provider = Arc::new(crate::llm::MockLLMProvider::new());
        let auth_service = AuthService::new(
            diesel::PgConnection::establish("postgres://user:pass@localhost:5432/db").unwrap(),
            None,
        );

        Self {
            s3_client: None,
            config: None,
            conn: Arc::new(Mutex::new(
                diesel::PgConnection::establish("postgres://user:pass@localhost:5432/db").unwrap(),
            )),
            redis_client: None,
            browser_pool: Arc::new(crate::web_automation::BrowserPool::new(
                "chrome".to_string(),
                2,
                "headless".to_string(),
            )),
            orchestrator: Arc::new(BotOrchestrator::new(
                session_manager,
                tool_manager,
                llm_provider,
                auth_service,
            )),
            web_adapter: Arc::new(WebChannelAdapter::new()),
            voice_adapter: Arc::new(VoiceAdapter::new(
                "https://livekit.example.com".to_string(),
                "api_key".to_string(),
                "api_secret".to_string(),
            )),
            whatsapp_adapter: Arc::new(WhatsAppAdapter::new(
                "whatsapp_token".to_string(),
                "phone_number_id".to_string(),
                "verify_token".to_string(),
            )),
            tool_api: Arc::new(ToolApi::new()),
        }
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            s3_client: self.s3_client.clone(),
            config: self.config.clone(),
            conn: Arc::clone(&self.conn),
            redis_client: self.redis_client.clone(),
            browser_pool: Arc::clone(&self.browser_pool),
            orchestrator: Arc::clone(&self.orchestrator),
            web_adapter: Arc::clone(&self.web_adapter),
            voice_adapter: Arc::clone(&self.voice_adapter),
            whatsapp_adapter: Arc::clone(&self.whatsapp_adapter),
            tool_api: Arc::clone(&self.tool_api),
        }
    }
}
