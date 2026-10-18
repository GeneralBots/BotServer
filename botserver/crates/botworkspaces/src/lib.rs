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

// Canonical definitions live in `botschema::workspaces` (#1497).
pub use botschema::{
    aiworkspace_comments, aiworkspace_members, aiworkspace_page_versions, aiworkspace_pages,
    aiworkspaces,
};

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

