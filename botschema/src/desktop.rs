use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

diesel::table! {
    desktop_connections (id) {

    id -> Uuid,
    user_id -> Uuid,
    organization_id -> Nullable<Uuid>,
    #[max_length = 255]
    name -> Varchar,
    #[max_length = 255]
    host -> Varchar,
    port -> Int4,
    #[max_length = 10]
    protocol -> Varchar,
    #[max_length = 20]
    auth_type -> Nullable<Varchar>,
    auto_connect -> Bool,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    last_used_at -> Nullable<Timestamptz>,
    #[max_length = 500]
    secrets_vault_path -> Nullable<Varchar>,
    #[max_length = 255]
    rdp_domain -> Nullable<Varchar>,
    }
}


diesel::table! {
    desktop_connection_log (id) {
        id -> Uuid,
        connection_id -> Nullable<Uuid>,
        user_id -> Uuid,
        session_id -> Uuid,
        host -> Varchar,
        port -> Int4,
        protocol -> Varchar,
        connected_at -> Timestamptz,
        disconnected_at -> Nullable<Timestamptz>,
        bytes_transferred -> Nullable<Int8>,
        disconnect_reason -> Nullable<Varchar>,
    }
}

#[derive(Queryable, Debug, Clone, Selectable)]
#[diesel(table_name = desktop_connections)]
pub struct DesktopConnectionDb {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub protocol: String,
    pub auth_type: Option<String>,
    pub auto_connect: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = desktop_connections)]
pub struct NewDesktopConnection {
    pub user_id: Uuid,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub protocol: String,
    pub auth_type: Option<String>,
    pub auto_connect: Option<bool>,
}

#[derive(Queryable, Debug, Clone, Selectable)]
#[diesel(table_name = desktop_connection_log)]
pub struct DesktopConnectionLogDb {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub host: String,
    pub port: i32,
    pub protocol: String,
    pub connected_at: DateTime<Utc>,
    pub disconnected_at: Option<DateTime<Utc>>,
    pub bytes_transferred: Option<i64>,
    pub disconnect_reason: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = desktop_connection_log)]
pub struct NewDesktopConnectionLog {
    pub connection_id: Option<Uuid>,
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub host: String,
    pub port: i32,
    pub protocol: String,
}

diesel::joinable!(desktop_connection_log -> desktop_connections (connection_id));
allow_tables_to_appear_in_same_query!(desktop_connections, desktop_connection_log);
