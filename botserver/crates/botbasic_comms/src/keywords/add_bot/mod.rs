pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{info, trace};
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TriggerType {
    Keyword,
    Tool,
    Schedule,
    Event,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotTrigger {
    pub trigger_type: TriggerType,
    pub keywords: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub schedule: Option<String>,
    pub event_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBot {
    pub id: Uuid,
    pub session_id: Uuid,
    pub bot_id: Uuid,
    pub bot_name: String,
    pub trigger: BotTrigger,
    pub priority: i32,
    pub is_active: bool,
}

#[derive(QueryableByName)]
struct BoolResult {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    exists: bool,
}

#[derive(QueryableByName)]
struct UuidResult {
    #[diesel(sql_type = diesel::sql_types::Text)]
    id: String,
}

#[derive(QueryableByName)]
struct SessionBotRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    session_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    bot_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    bot_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    trigger_config: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    priority: i32,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    is_active: bool,
}

#[derive(QueryableByName)]
struct BotConfigRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    system_prompt: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    model_config: Option<String>,
}

use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part3::*;
use part4::*;
pub use part5::*;
