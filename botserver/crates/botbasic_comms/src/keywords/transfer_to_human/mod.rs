pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{debug, info, warn};
pub(crate) use rhai::{Dynamic, Engine, Map};
pub(crate) use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferToHumanRequest {
    pub name: Option<String>,

    pub department: Option<String>,

    pub priority: Option<String>,

    pub reason: Option<String>,

    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub success: bool,
    pub status: TransferStatus,
    pub queue_position: Option<i32>,
    pub assigned_to: Option<String>,
    pub assigned_to_name: Option<String>,
    pub estimated_wait_seconds: Option<i32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Queued,

    Assigned,

    Connected,

    NoAttendants,

    CrmDisabled,

    AttendantNotFound,

    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendant {
    pub id: String,
    pub name: String,
    pub channel: String,
    pub preferences: String,
    pub department: Option<String>,
    pub aliases: Vec<String>,
    pub status: AttendantStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AttendantStatus {
    Online,
    Busy,
    Away,
    #[default]
    Offline,
}

pub const TRANSFER_TO_HUMAN_TOOL_SCHEMA: &str = r#"{
    "name": "transfer_to_human",
    "description": "Transfer the conversation to a human attendant. Use this when the customer explicitly asks to speak with a person, when the issue is too complex for automated handling, or when emotional support is needed.",
    "parameters": {
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "If someone wants to talk to somebody specific, provide their name or alias. Leave empty for any available attendant."
            },
            "department": {
                "type": "string",
                "description": "Department to transfer to: sales, support, technical, billing, etc.",
                "enum": ["sales", "support", "technical", "billing", "general"]
            },
            "priority": {
                "type": "string",
                "description": "Priority level for the transfer",
                "enum": ["normal", "high", "urgent"],
                "default": "normal"
            },
            "reason": {
                "type": "string",
                "description": "Brief reason for the transfer to help the attendant understand the context"
            }
        },
        "required": []
    }
}"#;

use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
