use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub type GetBotContextFn = Arc<dyn Fn(&DbPool) -> (Uuid, Uuid) + Send + Sync>;

#[derive(Clone)]
pub struct CanvasState {
    pub pool: Arc<DbPool>,
    pub get_bot_context: GetBotContextFn,
}

diesel::table! {
    canvases (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        width -> Int4,
        height -> Int4,
        background_color -> Nullable<Varchar>,
        thumbnail_url -> Nullable<Text>,
        is_public -> Bool,
        is_template -> Bool,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

pub use botschema::canvas_elements;

pub use botschema::canvas_collaborators;

pub use botschema::canvas_versions;

pub use botschema::canvas_comments;




// #1370 — this crate root used to be a 2090-line file. It is split by
// responsibility: models (rows, domain types, payloads), api (canvas and
// element CRUD), export (PNG/SVG rendering), collaboration (collaborators,
// comments, versions) and ui/ui_views (HTMX fragments). The diesel `table!`
// macros and the route tables stay here so the schema module and public
// routes keep their existing paths.
mod api;
mod collaboration;
mod export;
mod models;
mod ui;
mod ui_views;

pub(crate) use api::*;
pub use collaboration::*;
pub(crate) use export::*;
pub use models::*;
pub use ui::*;
pub use ui_views::*;

pub fn configure_canvas_routes() -> Router<Arc<CanvasState>> {
    Router::new()
        .route(
            "/api/canvas/collaborators",
            get(list_workspace_collaborators),
        )
        .route("/api/canvas", get(list_canvases).post(create_canvas))
        .route(
            "/api/canvas/:canvas_id",
            get(get_canvas).put(update_canvas).delete(delete_canvas),
        )
        .route(
            "/api/canvas/:canvas_id/elements",
            get(list_elements).post(create_element),
        )
        .route(
            "/api/canvas/:canvas_id/elements/:element_id",
            put(update_element).delete(delete_element),
        )
        .route("/api/canvas/:canvas_id/export", post(export_canvas))
        .route(
            "/api/canvas/:canvas_id/collaborators",
            get(list_collaborators).post(add_collaborator),
        )
        .route(
            "/api/canvas/:canvas_id/collaborators/:user_id",
            axum::routing::delete(remove_collaborator),
        )
        .route(
            "/api/canvas/:canvas_id/comments",
            get(list_comments).post(create_comment),
        )
        .route(
            "/api/canvas/:canvas_id/comments/:comment_id/resolve",
            put(resolve_comment),
        )
        .route(
            "/api/canvas/:canvas_id/versions",
            get(list_versions).post(create_version),
        )
        .route(
            "/api/canvas/:canvas_id/collaborate",
            get(get_collaboration_info),
        )
}

pub fn configure_canvas_ui_routes() -> Router<Arc<CanvasState>> {
    Router::new()
        .route("/api/ui/canvas", get(canvas_list))
        .route("/api/ui/canvas/cards", get(canvas_cards))
        .route("/api/ui/canvas/count", get(canvas_count))
        .route("/api/ui/canvas/templates/count", get(canvas_templates_count))
        .route("/api/ui/canvas/new", get(new_canvas_form))
        .route("/api/ui/canvas/:canvas_id", get(canvas_detail))
        .route("/api/ui/canvas/:canvas_id/editor", get(canvas_editor))
        .route("/api/ui/canvas/:canvas_id/elements", get(canvas_elements_svg))
        .route("/api/ui/canvas/:canvas_id/settings", get(canvas_settings))
}
