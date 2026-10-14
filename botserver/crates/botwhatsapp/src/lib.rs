pub mod api;
pub mod media;
pub mod models;
pub mod schema;
pub mod session_management;
pub mod state;
pub mod utils;
pub mod webhooks;
pub mod message_processing;
mod media_names;

pub use state::WhatsAppState;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub const WHATSAPP_WEBHOOK: &str = "/api/whatsapp/webhook";
pub const WHATSAPP_WEBHOOK_VERIFY: &str = "/api/whatsapp/webhook";
pub const WHATSAPP_SEND: &str = "/api/whatsapp/send";
pub const WHATSAPP_STATUS: &str = "/api/whatsapp/status";
pub const WHATSAPP_SESSIONS: &str = "/api/whatsapp/sessions";

pub use botlib::db_pool::DbPool;

pub fn configure_whatsapp_routes() -> Router<Arc<WhatsAppState>> {
    Router::new()
        .route(WHATSAPP_WEBHOOK, post(webhooks::handle_webhook))
        .route(WHATSAPP_WEBHOOK_VERIFY, get(webhooks::handle_webhook_verify))
        .route(WHATSAPP_SEND, post(api::handle_send_message))
        .route(WHATSAPP_STATUS, get(api::handle_status))
        .route(WHATSAPP_SESSIONS, get(api::handle_sessions))
}
