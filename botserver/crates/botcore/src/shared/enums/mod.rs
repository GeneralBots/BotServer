//! Database Enum Types for Billion-Scale Schema
//!
//! This module defines Rust enums that map directly to PostgreSQL enum types.
//! Using enums instead of TEXT columns provides:
//! - Type safety at compile time
//! - Efficient storage (stored as integers internally)
//! - Fast comparisons and indexing
//! - Automatic validation
//!
//! All enums derive necessary traits for Diesel ORM integration.

// use diesel::{AsExpression, FromSqlRow}; // Removed to avoid conflict

// ============================================================================
// CHANNEL TYPES
// ============================================================================

use diesel::deserialize::{self, FromSql};
pub(crate) use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, Output, ToSql};
pub(crate) use diesel::sql_types::SmallInt;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::io::Write;

/// Communication channel types for bot interactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum ChannelType {
    #[default]
    Web = 0,
    WhatsApp = 1,
    Telegram = 2,
    MsTeams = 3,
    Slack = 4,
    Email = 5,
    Sms = 6,
    Voice = 7,
    Instagram = 8,
    Api = 9,
}

/// Role of a message in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum MessageRole {
    #[default]
    User = 1,
    Assistant = 2,
    System = 3,
    Tool = 4,
    Episodic = 9,
    Compact = 10,
}

/// Type of message content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum MessageType {
    #[default]
    Text = 0,
    Image = 1,
    Audio = 2,
    Video = 3,
    Document = 4,
    Location = 5,
    Contact = 6,
    Sticker = 7,
    Reaction = 8,
}

/// Supported LLM providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum LlmProvider {
    #[default]
    OpenAi = 0,
    Anthropic = 1,
    AzureOpenAi = 2,
    AzureClaude = 3,
    Google = 4,
    Local = 5,
    Ollama = 6,
    Groq = 7,
    Mistral = 8,
    Cohere = 9,
}

/// Supported vector database providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum ContextProvider {
    None = 0,
    #[default]
    Qdrant = 1,
    Pinecone = 2,
    Weaviate = 3,
    Milvus = 4,
    PgVector = 5,
    Elasticsearch = 6,
}

/// Status of a task (both regular tasks and auto-tasks)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum TaskStatus {
    #[default]
    Pending = 0,
    Ready = 1,
    Running = 2,
    Paused = 3,
    WaitingApproval = 4,
    Completed = 5,
    Failed = 6,
    Cancelled = 7,
}

/// Priority level for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum TaskPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Urgent = 3,
    Critical = 4,
}

/// Execution mode for autonomous tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum ExecutionMode {
    Manual = 0,
    #[default]
    Supervised = 1,
    Autonomous = 2,
}

/// Risk assessment level for actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum RiskLevel {
    None = 0,
    #[default]
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Status of an approval request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
#[derive(Default)]
pub enum ApprovalStatus {
    #[default]
    Pending = 0,
    Approved = 1,
    Rejected = 2,
    Expired = 3,
    Skipped = 4,
}

/// Decision made on an approval request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "snake_case")]
#[repr(i16)]
pub enum ApprovalDecision {
    Approve = 0,
    Reject = 1,
    Skip = 2,
}

/// Classified intent type from user requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(i16)]
#[derive(Default)]
pub enum IntentType {
    #[default]
    Unknown = 0,
    AppCreate = 1,
    Todo = 2,
    Monitor = 3,
    Action = 4,
    Schedule = 5,
    Goal = 6,
    Tool = 7,
    Query = 8,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;
mod part11;
mod part12;
