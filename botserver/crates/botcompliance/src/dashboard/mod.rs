//! Compliance dashboard HTMX fragments plus scan and export endpoints.

pub(crate) use axum::{ extract::State, response::{Html, IntoResponse}, routing::{get, post}, Json, Router, };
pub(crate) use diesel::prelude::*;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::DbPool;
pub(crate) use crate::schema::{compliance_checks, compliance_evidence};

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn branch_id_for(pool: &Arc<DbPool>) -> Uuid {
    let Ok(mut conn) = pool.get() else {
        return Uuid::nil();
    };
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct BranchRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        branch_id: Uuid,
    }
    diesel::sql_query(
        "SELECT branch_id FROM bots WHERE is_default_for_branch = TRUE ORDER BY created_at ASC LIMIT 1",
    )
    .get_result::<BranchRow>(&mut conn)
    .map(|r| r.branch_id)
    .unwrap_or(Uuid::nil())
}

pub fn configure_dashboard_routes() -> Router<Arc<DbPool>> {
    Router::new()
        .route("/api/compliance/dashboard/overview", get(handle_overview))
        .route("/api/compliance/dashboard/tsc", get(handle_tsc))
        .route("/api/compliance/dashboard/failing-controls", get(handle_failing_controls))
        .route("/api/compliance/dashboard/evidence", get(handle_evidence))
        .route("/api/compliance/dashboard/audit-log", get(handle_audit_log))
        .route("/api/compliance/scan", post(handle_scan))
        .route("/api/compliance/export", get(handle_export))
}

mod overview;
mod tsc;
mod lists;
mod actions;

use overview::*;
use tsc::*;
use lists::*;
use actions::*;
