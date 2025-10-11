use crate::bot::BotOrchestrator;
use crate::channels::{VoiceAdapter, WebChannelAdapter};
use crate::config::AppConfig;
use crate::tools::ToolApi;
use crate::whatsapp::WhatsAppAdapter;
use diesel::PgConnection;
use redis::Client;
use std::sync::Arc;
use std::sync::Mutex;

pub struct AppState {
    pub s3_client: Option<aws_sdk_s3::Client>,
    pub config: Option<AppConfig>,
    pub conn: Arc<Mutex<PgConnection>>,
    pub custom_conn: Arc<Mutex<PgConnection>>,

    pub redis_client: Option<Arc<Client>>,
    pub orchestrator: Arc<BotOrchestrator>,
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
            orchestrator: Arc::clone(&self.orchestrator),
            web_adapter: Arc::clone(&self.web_adapter),
            voice_adapter: Arc::clone(&self.voice_adapter),
            whatsapp_adapter: Arc::clone(&self.whatsapp_adapter),
            tool_api: Arc::clone(&self.tool_api),
        }
    }
}
