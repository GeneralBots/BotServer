use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

diesel::table! {
    database_query_history (id) {

    id -> Uuid,
    bot_id -> Uuid,
    user_id -> Uuid,
    query_text -> Text,
    is_mutation -> Bool,
    row_count -> Nullable<Int8>,
    duration_ms -> Nullable<Int8>,
    error_message -> Nullable<Text>,
    executed_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    database_saved_queries (id) {

    id -> Uuid,
    bot_id -> Uuid,
    user_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    query_text -> Text,
    description -> Nullable<Text>,
    is_shared -> Nullable<Bool>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


#[derive(Queryable, Debug, Clone, Selectable)]
#[diesel(table_name = database_query_history)]
pub struct QueryHistoryDb {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub user_id: Uuid,
    pub query_text: String,
    pub is_mutation: bool,
    pub row_count: Option<i64>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
    pub executed_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = database_query_history)]
pub struct NewQueryHistory {
    pub bot_id: Uuid,
    pub user_id: Uuid,
    pub query_text: String,
    pub is_mutation: bool,
    pub row_count: Option<i64>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
    pub branch_id: Uuid,
}

#[derive(Queryable, Debug, Clone, Selectable)]
#[diesel(table_name = database_saved_queries)]
pub struct SavedQueryDb {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub query_text: String,
    pub description: Option<String>,
    pub is_shared: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = database_saved_queries)]
pub struct NewSavedQuery {
    pub bot_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub query_text: String,
    pub description: Option<String>,
    pub is_shared: Option<bool>,
    pub branch_id: Uuid,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = database_saved_queries)]
pub struct SavedQueryUpdate {
    pub name: Option<String>,
    pub query_text: Option<String>,
    pub description: Option<String>,
    pub is_shared: Option<bool>,
    pub updated_at: DateTime<Utc>,
}
