use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

diesel::table! {
    drive_files (id) {

    id -> Uuid,
    file_path -> Text,
    #[max_length = 20]
    file_type -> Varchar,
    etag -> Nullable<Text>,
    last_modified -> Nullable<Timestamptz>,
    file_size -> Nullable<Int8>,
    indexed -> Bool,
    fail_count -> Int4,
    last_failed_at -> Nullable<Timestamptz>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    user_id -> Nullable<Uuid>,
    #[max_length = 20]
    scope -> Nullable<Varchar>,
    branch_id -> Uuid,
    path -> Nullable<Text>,
    name -> Nullable<Text>,
    #[max_length = 50]
    mime_type -> Nullable<Varchar>,
    }
}


// Query-only struct (no defaults needed)
#[derive(Queryable, Debug, Clone)]
pub struct DriveFile {
    pub id: Uuid,
    pub file_path: String,
    pub file_type: String,
    pub etag: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub file_size: Option<i64>,
    pub indexed: bool,
    pub fail_count: i32,
    pub last_failed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub scope: Option<String>,
    pub branch_id: Uuid,
    pub path: Option<String>,
    pub name: Option<String>,
    pub mime_type: Option<String>,
}

// Insert struct - uses diesel defaults
#[derive(Insertable, Debug)]
#[diesel(table_name = drive_files)]
pub struct NewDriveFile {
    pub file_path: String,
    pub file_type: String,
    pub etag: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub file_size: Option<i64>,
    pub indexed: Option<bool>,
    pub fail_count: Option<i32>,
}

// Update struct
#[derive(AsChangeset, Debug)]
#[diesel(table_name = drive_files)]
pub struct DriveFileUpdate {
    pub etag: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub file_size: Option<i64>,
    pub indexed: Option<bool>,
    pub fail_count: Option<i32>,
    pub last_failed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

// ====== Per-user file scoping (Issue #589) ======

diesel::table! {
    drive_user_permissions (id) {
        id -> Uuid,
        user_id -> Uuid,
        bucket -> Varchar,
        path -> Text,
        permission -> Varchar,
        granted_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        expires_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    drive_starred (id) {
        id -> Uuid,
        user_id -> Uuid,
        bucket -> Varchar,
        path -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    drive_share_links (id) {

    id -> Uuid,
    created_by -> Uuid,
    #[max_length = 255]
    bucket -> Varchar,
    path -> Text,
    #[max_length = 64]
    token -> Varchar,
    #[max_length = 20]
    permission -> Varchar,
    expires_at -> Nullable<Timestamptz>,
    max_downloads -> Nullable<Int4>,
    download_count -> Nullable<Int4>,
    created_at -> Timestamptz,
    }
}


#[derive(Queryable, Debug, Clone, Selectable)]
#[diesel(table_name = drive_user_permissions)]
pub struct DriveUserPermission {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bucket: String,
    pub path: String,
    pub permission: String,
    pub granted_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = drive_user_permissions)]
pub struct NewDriveUserPermission {
    pub user_id: Uuid,
    pub bucket: String,
    pub path: String,
    pub permission: String,
    pub granted_by: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Debug, Clone, Selectable)]
#[diesel(table_name = drive_starred)]
pub struct DriveStarred {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bucket: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = drive_starred)]
pub struct NewDriveStarred {
    pub user_id: Uuid,
    pub bucket: String,
    pub path: String,
}

#[derive(Queryable, Debug, Clone, Selectable)]
#[diesel(table_name = drive_share_links)]
pub struct DriveShareLink {
    pub id: Uuid,
    pub created_by: Uuid,
    pub bucket: String,
    pub path: String,
    pub token: String,
    pub permission: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_downloads: Option<i32>,
    pub download_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = drive_share_links)]
pub struct NewDriveShareLink {
    pub created_by: Uuid,
    pub bucket: String,
    pub path: String,
    pub token: String,
    pub permission: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_downloads: Option<i32>,
}
