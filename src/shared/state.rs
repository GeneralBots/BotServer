use crate::auth::AuthService;
use crate::channels::{ChannelAdapter, VoiceAdapter, WebChannelAdapter};
use crate::config::AppConfig;
use crate::llm::LLMProvider;
use crate::session::SessionManager;
use crate::tools::{ToolApi, ToolManager};
use crate::whatsapp::WhatsAppAdapter;
use diesel::{Connection, PgConnection};
use redis::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;

use crate::shared::models::BotResponse;

pub struct AppState {
    pub s3_client: Option<aws_sdk_s3::Client>,
    pub config: Option<AppConfig>,
    pub conn: Arc<Mutex<PgConnection>>,
    pub custom_conn: Arc<Mutex<PgConnection>>,
    pub redis_client: Option<Arc<Client>>,

    pub session_manager: Arc<tokio::sync::Mutex<SessionManager>>,
    pub tool_manager: Arc<ToolManager>,
    pub llm_provider: Arc<dyn LLMProvider>,
    pub auth_service: Arc<tokio::sync::Mutex<AuthService>>,
    pub channels: Arc<Mutex<HashMap<String, Arc<dyn ChannelAdapter>>>>,
    pub response_channels: Arc<tokio::sync::Mutex<HashMap<String, mpsc::Sender<BotResponse>>>>,

    pub web_adapter: Arc<WebChannelAdapter>,
    pub voice_adapter: Arc<VoiceAdapter>,
    pub whatsapp_adapter: Arc<WhatsAppAdapter>,
    pub tool_api: Arc<ToolApi>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            s3_client: self.s3_client.clone(),
            config: self.config.clone(),
            conn: Arc::clone(&self.conn),
            custom_conn: Arc::clone(&self.custom_conn),
            redis_client: self.redis_client.clone(),
            session_manager: Arc::clone(&self.session_manager),
            tool_manager: Arc::clone(&self.tool_manager),
            llm_provider: Arc::clone(&self.llm_provider),
            auth_service: Arc::clone(&self.auth_service),
            channels: Arc::clone(&self.channels),
            response_channels: Arc::clone(&self.response_channels),
            web_adapter: Arc::clone(&self.web_adapter),
            voice_adapter: Arc::clone(&self.voice_adapter),
            whatsapp_adapter: Arc::clone(&self.whatsapp_adapter),
            tool_api: Arc::clone(&self.tool_api),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            s3_client: None,
            config: None,
            conn: Arc::new(Mutex::new(
                diesel::PgConnection::establish("postgres://localhost/test").unwrap(),
            )),
            custom_conn: Arc::new(Mutex::new(
                diesel::PgConnection::establish("postgres://localhost/test").unwrap(),
            )),
            redis_client: None,
            session_manager: Arc::new(tokio::sync::Mutex::new(SessionManager::new(
                diesel::PgConnection::establish("postgres://localhost/test").unwrap(),
                None,
            ))),
            tool_manager: Arc::new(ToolManager::new()),
            llm_provider: Arc::new(crate::llm::OpenAIClient::new(
                "empty".to_string(),
                Some("http://localhost:8081".to_string()),
            )),
            auth_service: Arc::new(tokio::sync::Mutex::new(AuthService::new(
                diesel::PgConnection::establish("postgres://localhost/test").unwrap(),
                None,
            ))),
            channels: Arc::new(Mutex::new(HashMap::new())),
            response_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
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
