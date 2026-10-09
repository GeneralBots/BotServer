//! ON EMAIL event handler keywords, monitor polling and email path helpers.

pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use botlib::models::TriggerKind;
pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{info, trace};
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMonitor {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub email_address: String,
    pub script_path: String,
    pub is_active: bool,
    pub filter_from: Option<String>,
    pub filter_subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailReceivedEvent {
    pub id: Uuid,
    pub monitor_id: Uuid,
    pub message_uid: i64,
    pub message_id: Option<String>,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub subject: Option<String>,
    pub has_attachments: bool,
    pub attachments: Vec<EmailAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
}

mod registration;
mod monitors;
mod paths;

pub use registration::*;
pub use monitors::*;
pub use paths::*;
