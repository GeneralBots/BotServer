pub(crate) use base64::Engine;
pub(crate) use crate::connector_types::*;
pub(crate) use chrono::Utc;
pub(crate) use diesel::sql_query;
pub(crate) use diesel::prelude::*;
pub(crate) use uuid::Uuid;

pub struct ConnectorEngine;

#[derive(diesel::QueryableByName)]
struct ConnectorRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)] bot_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)] name: String,
    #[diesel(sql_type = diesel::sql_types::Text)] connector_type: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] description: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Jsonb)] auth_config: serde_json::Value,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] schedule: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Bool)] is_active: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)] last_sync_at: Option<chrono::DateTime<Utc>>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] last_sync_status: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] error_log: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)] updated_at: chrono::DateTime<Utc>,
}

mod part1;
mod part2;
mod part3;

use part3::*;
