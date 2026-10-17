use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub mod scope;

diesel::table! {
    aiworkspaces (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        icon_type -> Nullable<Varchar>,
        icon_value -> Nullable<Varchar>,
        cover_image -> Nullable<Text>,
        settings -> Jsonb,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    aiworkspace_members (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        user_id -> Uuid,
        role -> Varchar,
        invited_by -> Nullable<Uuid>,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    aiworkspace_pages (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        parent_id -> Nullable<Uuid>,
        title -> Varchar,
        icon_type -> Nullable<Varchar>,
        icon_value -> Nullable<Varchar>,
        cover_image -> Nullable<Text>,
        content -> Jsonb,
        properties -> Jsonb,
        is_template -> Bool,
        template_id -> Nullable<Uuid>,
        is_public -> Bool,
        public_edit -> Bool,
        position -> Int4,
        created_by -> Uuid,
        last_edited_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    aiworkspace_page_versions (id) {
        id -> Uuid,
        page_id -> Uuid,
        version_number -> Int4,
        title -> Varchar,
        content -> Jsonb,
        change_summary -> Nullable<Text>,
        created_by -> Uuid,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    aiworkspace_comments (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        page_id -> Uuid,
        block_id -> Nullable<Uuid>,
        parent_comment_id -> Nullable<Uuid>,
        author_id -> Uuid,
        content -> Text,
        resolved -> Bool,
        resolved_by -> Nullable<Uuid>,
        resolved_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    aiworkspaces,
    aiworkspace_members,
    aiworkspace_pages,
    aiworkspace_page_versions,
    aiworkspace_comments,
);

pub type GetDefaultBotFn = fn(&mut diesel::PgConnection) -> Uuid;

#[derive(Debug, Clone)]
pub struct WorkspacesState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

fn get_bot_context(state: &WorkspacesState) -> Uuid {
    let Ok(mut conn) = state.pool.get() else {
        return Uuid::nil();
    };
    let bid: Uuid = bots::table
        .filter(bots::is_default_for_branch.eq(true))
        .order(bots::created_at.asc())
        .select(bots::branch_id)
        .first(&mut conn)
        .unwrap_or(Uuid::nil());
    bid
}


// #1370 — this crate root used to be a 4055-line file. It is split by
// responsibility: models (DB rows), domain (API types), dto (converters and
// request payloads), handlers (REST), pages/page_ops (page logic), blocks /
// block_ops (block builder), collaboration / transform (realtime editing),
// ui* (HTMX fragments). The public surface is unchanged: the glob re-exports
// below expose exactly the items that were public before the split.
mod block_ops;
mod blocks;
mod collaboration;
mod domain;
mod dto;
mod handlers;
mod handlers_pages;
mod models;
mod page_ops;
mod pages;
mod transform;
mod ui;
mod ui_current;
mod ui_forms;
mod ui_views;

pub use block_ops::*;
pub use blocks::*;
pub use collaboration::*;
pub use domain::*;
pub use dto::*;
// The REST handlers were private before the split; keep them crate-visible.
pub(crate) use handlers::*;
pub(crate) use handlers_pages::*;
pub use models::*;
pub use page_ops::*;
pub use pages::*;
pub use transform::*;
pub use ui::*;
pub use ui_current::*;
pub use ui_forms::*;
pub use ui_views::*;
pub fn configure_workspaces_routes() -> Router<Arc<WorkspacesState>> {
    Router::new()
        .route("/api/workspaces", get(list_workspaces).post(create_workspace))
        .route(
            "/api/workspaces/:workspace_id",
            get(get_workspace).put(update_workspace).delete(delete_workspace),
        )
        .route(
            "/api/workspaces/:workspace_id/pages",
            get(list_pages).post(create_page),
        )
        .route("/api/workspaces/:workspace_id/members", post(add_member))
        .route(
            "/api/workspaces/:workspace_id/members/:user_id",
            delete(remove_member),
        )
        .route("/api/workspaces/:workspace_id/search", get(search_pages))
        .route(
            "/api/pages/:page_id",
            get(get_page).put(update_page).delete(delete_page),
        )
        .route("/api/workspaces/commands", get(get_slash_commands_handler))
}

