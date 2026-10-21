use axum::{
    extract::State,
    response::Html,
    routing::get,
    Router,
};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::schema::*;
use crate::AttendantConfig;

pub mod dashboard;
pub mod sessions;

#[derive(Debug, Deserialize, Default)]
pub struct SessionListQuery {
    pub status: Option<String>,
    pub queue_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn sessions_count(State(config): State<Arc<AttendantConfig>>) -> Html<String> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        attendant_sessions::table
            .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(result.unwrap_or(0).to_string())
}

pub async fn waiting_count(State(config): State<Arc<AttendantConfig>>) -> Html<String> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        attendant_sessions::table
            .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
            .filter(attendant_sessions::status.eq("waiting"))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(result.unwrap_or(0).to_string())
}

pub async fn active_count(State(config): State<Arc<AttendantConfig>>) -> Html<String> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        attendant_sessions::table
            .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
            .filter(attendant_sessions::status.eq("active"))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(result.unwrap_or(0).to_string())
}

pub async fn agents_online_count(State(config): State<Arc<AttendantConfig>>) -> Html<String> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        attendant_agent_status::table
            .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
            .filter(attendant_agent_status::status.eq("online"))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(result.unwrap_or(0).to_string())
}

pub fn configure_attendant_ui_routes() -> Router<Arc<AttendantConfig>> {
    Router::new()
        .route("/api/ui/attendant/sessions", get(sessions::sessions_table))
        .route("/api/ui/attendant/sessions/count", get(sessions_count))
        .route("/api/ui/attendant/sessions/waiting", get(waiting_count))
        .route("/api/ui/attendant/sessions/active", get(active_count))
        .route("/api/ui/attendant/sessions/:id", get(sessions::session_detail))
        .route("/api/ui/attendant/queues", get(sessions::queues_list))
        .route("/api/ui/attendant/queues/:id/stats", get(sessions::queue_stats))
        .route("/api/ui/attendant/agents", get(sessions::agent_status_list))
        .route("/api/ui/attendant/agents/online", get(agents_online_count))
        .route("/api/ui/attendant/dashboard", get(dashboard::dashboard_stats))
}
