pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{info, trace, warn};
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum ReflectionType {
    #[default]
    ConversationQuality,
    ResponseAccuracy,
    ToolUsage,
    KnowledgeRetrieval,
    Performance,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionConfig {
    pub enabled: bool,

    pub interval: u32,

    pub custom_prompt: Option<String>,

    pub auto_apply: bool,

    pub improvement_threshold: f32,

    pub max_insights: usize,

    pub reflection_types: Vec<ReflectionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResult {
    pub id: Uuid,

    pub bot_id: Uuid,

    pub session_id: Uuid,

    pub reflection_type: ReflectionType,

    pub score: f32,

    pub insights: Vec<String>,

    pub improvements: Vec<String>,

    pub positive_patterns: Vec<String>,

    pub concerns: Vec<String>,

    pub raw_response: String,

    pub timestamp: chrono::DateTime<chrono::Utc>,

    pub messages_analyzed: usize,
}

pub struct ReflectionEngine {
    state: Arc<dyn BasicRuntime>,
    config: ReflectionConfig,
    bot_id: Uuid,
}

#[derive(Debug, Clone)]
struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;

pub use part3::*;
pub use part7::*;
use part8::*;
