pub mod schema;
pub mod state;
pub mod types;
pub mod scheduler;
pub mod scheduler_exec;
pub mod task_api;

pub use state::TasksState;
pub use task_api::TaskEngine;
pub use types::{AutoTask, NewAutoTask, TaskManifest};

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use task_api::handlers::*;
use task_api::ui_fragments::*;

pub const TASKS_LIST: &str = "/api/tasks";
pub const TASKS_CREATE: &str = "/api/tasks";
pub const TASKS_BY_ID: &str = "/api/tasks/{id}";
pub const TASKS_EXECUTE: &str = "/api/tasks/{id}/execute";
pub const TASKS_CANCEL: &str = "/api/tasks/{id}/cancel";
pub const TASKS_COMPLETE: &str = "/api/tasks/{id}/complete";
pub const TASKS_REOPEN: &str = "/api/tasks/{id}/reopen";
pub const TASKS_MANIFEST: &str = "/api/tasks/{id}/manifest";
pub const TASKS_CARDS: &str = "/api/tasks/{id}/cards";
pub const TASKS_TERMINAL: &str = "/api/tasks/{id}/terminal";
pub const TASKS_LOG: &str = "/api/tasks/{id}/log";

pub use botlib::db_pool::DbPool;

pub fn configure_tasks_routes() -> Router<Arc<TasksState>> {
    Router::new()
        .route(TASKS_LIST, get(handle_list_tasks))
        .route(TASKS_CREATE, post(handle_create_task))
        .route(
            TASKS_BY_ID,
            get(handle_get_task)
                .patch(handle_update_task)
                .delete(handle_delete_task),
        )
        .route(TASKS_EXECUTE, post(handle_execute_task))
        .route(TASKS_CANCEL, post(handle_cancel_task))
        .route(TASKS_COMPLETE, post(handle_complete_task))
        .route(TASKS_REOPEN, post(handle_reopen_task))
        .route(TASKS_MANIFEST, get(handle_get_manifest))
        .route(TASKS_CARDS, get(handle_get_cards))
        .route(TASKS_TERMINAL, get(handle_get_terminal))
        .route(TASKS_LOG, get(handle_get_log))
        // HTMX fragment routes for the Tasks app UI
        .route("/api/ui/tasks", get(handle_ui_tasks_list))
        .route("/api/ui/tasks/stats", get(handle_ui_tasks_stats))
}
