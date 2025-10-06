use std::sync::Arc;

use crate::{
    bot::BotOrchestrator, 
    channels::{VoiceAdapter, WebChannelAdapter, WhatsAppAdapter},
    config::AppConfig,
    tools::ToolApi,
    web_automation::BrowserPool
};

#[derive(Clone)]
pub struct AppState {
    pub minio_client: Option<minio::s3::Client>,
    pub config: Option<AppConfig>,
    pub db: Option<sqlx::PgPool>,
    pub db_custom: Option<sqlx::PgPool>,
    pub browser_pool: Arc<BrowserPool>,
    pub orchestrator: Arc<BotOrchestrator>,
    pub web_adapter: Arc<WebChannelAdapter>,
    pub voice_adapter: Arc<VoiceAdapter>,
    pub whatsapp_adapter: Arc<WhatsAppAdapter>,
    pub tool_api: Arc<ToolApi>,
}

pub struct BotState {
    pub language: String,
    pub work_folder: String,
}
