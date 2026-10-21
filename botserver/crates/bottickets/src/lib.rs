

pub mod ui;
pub mod scope;
pub mod itsm;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub type GetDefaultBotFn = fn(&mut diesel::PgConnection) -> Uuid;

pub struct TicketsState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
}

pub use botschema::support_tickets;

pub use botschema::ticket_comments;

pub use botschema::ticket_sla_policies;

pub use botschema::ticket_canned_responses;

pub use botschema::ticket_categories;

pub use botschema::ticket_tags;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = support_tickets)]
pub struct SupportTicket {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub ticket_number: String,
    pub subject: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub category: Option<String>,
    pub source: String,
    pub requester_id: Option<Uuid>,
    pub requester_email: Option<String>,
    pub requester_name: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub due_date: Option<DateTime<Utc>>,
    pub first_response_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub satisfaction_rating: Option<i32>,
    pub tags: Vec<Option<String>>,
    pub custom_fields: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
    pub record_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = ticket_comments)]
pub struct TicketComment {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub author_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub content: String,
    pub is_internal: bool,
    pub attachments: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = ticket_sla_policies)]
pub struct TicketSlaPolicy {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub priority: String,
    pub first_response_hours: i32,
    pub resolution_hours: i32,
    pub business_hours_only: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = ticket_canned_responses)]
pub struct TicketCannedResponse {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub shortcut: Option<String>,
    pub created_by: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = ticket_categories)]
pub struct TicketCategory {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = ticket_tags)]
pub struct TicketTag {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTicketRequest {
    pub subject: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub requester_email: Option<String>,
    pub requester_name: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
    pub record_type: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTicketRequest {
    pub subject: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
    pub due_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignTicketRequest {
    pub assignee_id: Option<Uuid>,
    pub assignee: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeStatusRequest {
    pub status: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub is_internal: Option<bool>,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCannedResponseRequest {
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub shortcut: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub requester_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TicketStats {
    pub total_tickets: i64,
    pub open_tickets: i64,
    pub pending_tickets: i64,
    pub resolved_tickets: i64,
    pub closed_tickets: i64,
    pub avg_resolution_hours: f64,
    pub overdue_tickets: i64,
}

#[derive(Debug, Serialize)]
pub struct TicketWithComments {
    pub ticket: SupportTicket,
    pub comments: Vec<TicketComment>,
}

pub use botschema::bots;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TicketActivity {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_name: String,
    pub activity_type: String,
    pub description: String,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AddActivityRequest {
    pub activity_type: String,
    pub description: String,
    pub actor_name: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiSuggestion {
    pub ticket_id: Uuid,
    pub suggested_category: String,
    pub suggested_priority: String,
    pub suggested_assignee: Option<Uuid>,
    pub suggested_tags: Vec<String>,
    pub similar_tickets: Vec<Uuid>,
    pub confidence: f64,
    pub reasoning: String,
}

mod lib_part1;
mod lib_part2;
mod lib_part3;
mod lib_part4;
mod lib_part5;

pub use lib_part1::*;
pub use lib_part2::*;
pub use lib_part3::*;
pub use lib_part4::*;
pub use lib_part5::*;
