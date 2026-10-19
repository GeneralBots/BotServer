use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::shared::schema::core::workflow_executions, check_for_backend(diesel::pg::Pg))]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub workflow_name: String,
    pub current_step: i32,
    pub state_json: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// NOT NULL since `6.5.23-branch-scope-cleanup`.
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::shared::schema::core::workflow_events, check_for_backend(diesel::pg::Pg))]
pub struct WorkflowEvent {
    pub id: Uuid,
    /// Channel events carry the publishing bot here (see
    /// `basic_events::publish`); workflow-scoped events leave it `None`.
    pub workflow_id: Option<Uuid>,
    pub event_name: String,
    /// Serialized JSON — the column is TEXT.
    pub event_data_json: Option<String>,
    pub processed: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::shared::schema::core::bot_shared_memory, check_for_backend(diesel::pg::Pg))]
pub struct BotSharedMemory {
    pub id: Uuid,
    pub source_bot_id: Uuid,
    pub target_bot_id: Uuid,
    pub memory_key: String,
    pub memory_value: String,
    pub shared_at: chrono::DateTime<chrono::Utc>,
}
