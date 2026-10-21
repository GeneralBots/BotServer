use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::DashboardsError;
use crate::schema::{dashboard_filters, dashboard_widgets, dashboards};
use crate::storage::{
    db_dashboard_to_dashboard, db_filter_to_filter, db_widget_to_widget, DbDashboard, DbFilter,
    DbWidget,
};
use crate::types::{CreateDashboardRequest, Dashboard, DashboardFilter, ListDashboardsQuery, UpdateDashboardRequest, Widget};
use crate::DashboardsState;

pub async fn handle_list_dashboards(
    State(state): State<Arc<DashboardsState>>,
    Query(query): Query<ListDashboardsQuery>,
) -> Result<Json<Vec<Dashboard>>, DashboardsError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;
        let branch_id = get_default_bot(&mut conn);

        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let mut db_query = dashboards::table
            .filter(dashboards::branch_id.eq(&branch_id))
            .into_boxed();

        if let Some(owner_id) = query.owner_id {
            db_query = db_query.filter(dashboards::owner_id.eq(owner_id));
        }

        if let Some(is_template) = query.is_template {
            db_query = db_query.filter(dashboards::is_template.eq(is_template));
        }

        if let Some(ref search) = query.search {
            let term = format!("%{search}%");
            db_query = db_query.filter(dashboards::name.ilike(term));
        }

        let db_dashboards: Vec<DbDashboard> = db_query
            .order(dashboards::created_at.desc())
            .offset(offset)
            .limit(limit)
            .load(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        let mut result_dashboards = Vec::new();
        for db_dash in db_dashboards {
            let dash_id = db_dash.id;
            let widgets_db: Vec<DbWidget> = dashboard_widgets::table
                .filter(dashboard_widgets::dashboard_id.eq(dash_id))
                .load(&mut conn)
                .unwrap_or_default();
            let filters_db: Vec<DbFilter> = dashboard_filters::table
                .filter(dashboard_filters::dashboard_id.eq(dash_id))
                .load(&mut conn)
                .unwrap_or_default();

            let widgets: Vec<Widget> = widgets_db.into_iter().map(db_widget_to_widget).collect();
            let filters: Vec<DashboardFilter> =
                filters_db.into_iter().map(db_filter_to_filter).collect();

            result_dashboards.push(db_dashboard_to_dashboard(db_dash, widgets, filters));
        }

        Ok::<Vec<Dashboard>, DashboardsError>(result_dashboards)
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}

pub async fn handle_create_dashboard(
    State(state): State<Arc<DashboardsState>>,
    Json(req): Json<CreateDashboardRequest>,
) -> Result<Json<Dashboard>, DashboardsError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;
        let branch_id = get_default_bot(&mut conn);
        let now = Utc::now();

        let layout = req.layout.unwrap_or_default();
        let layout_json = serde_json::to_value(&layout).unwrap_or_default();

        let db_dashboard = DbDashboard {
            id: Uuid::new_v4(),
            org_id: Uuid::nil(),
            bot_id: Uuid::nil(),
            owner_id: Uuid::nil(),
            name: req.name,
            description: None,
            layout: layout_json,
            refresh_interval: None,
            is_public: req.is_public.unwrap_or(false),
            is_template: false,
            tags: req.tags.unwrap_or_default().into_iter().map(Some).collect(),
            created_at: now,
            updated_at: now,
            branch_id,
        };

        diesel::insert_into(dashboards::table)
            .values(&db_dashboard)
            .execute(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        Ok::<Dashboard, DashboardsError>(db_dashboard_to_dashboard(db_dashboard, vec![], vec![]))
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}

pub async fn handle_get_dashboard(
    State(state): State<Arc<DashboardsState>>,
    Path(dashboard_id): Path<Uuid>,
) -> Result<Json<Option<Dashboard>>, DashboardsError> {
    let pool = state.pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;

        let db_dash: Option<DbDashboard> = dashboards::table
            .find(dashboard_id)
            .first(&mut conn)
            .optional()
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        match db_dash {
            Some(db) => {
                let widgets_db: Vec<DbWidget> = dashboard_widgets::table
                    .filter(dashboard_widgets::dashboard_id.eq(dashboard_id))
                    .load(&mut conn)
                    .unwrap_or_default();
                let filters_db: Vec<DbFilter> = dashboard_filters::table
                    .filter(dashboard_filters::dashboard_id.eq(dashboard_id))
                    .load(&mut conn)
                    .unwrap_or_default();

                let widgets: Vec<Widget> = widgets_db.into_iter().map(db_widget_to_widget).collect();
                let filters: Vec<DashboardFilter> =
                    filters_db.into_iter().map(db_filter_to_filter).collect();

                Ok::<Option<Dashboard>, DashboardsError>(Some(db_dashboard_to_dashboard(db, widgets, filters)))
            }
            None => Ok(None),
        }
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}

pub async fn handle_update_dashboard(
    State(state): State<Arc<DashboardsState>>,
    Path(dashboard_id): Path<Uuid>,
    Json(req): Json<UpdateDashboardRequest>,
) -> Result<Json<Dashboard>, DashboardsError> {
    let pool = state.pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;

        let mut db_dash: DbDashboard = dashboards::table
            .find(dashboard_id)
            .first(&mut conn)
            .map_err(|_| DashboardsError::NotFound("Dashboard not found".to_string()))?;

        if let Some(name) = req.name {
            db_dash.name = name;
        }
        if let Some(description) = req.description {
            db_dash.description = Some(description);
        }
        if let Some(layout) = req.layout {
            db_dash.layout = serde_json::to_value(&layout).unwrap_or_default();
        }
        if let Some(is_public) = req.is_public {
            db_dash.is_public = is_public;
        }
        if let Some(refresh_interval) = req.refresh_interval {
            db_dash.refresh_interval = Some(refresh_interval);
        }
        if let Some(tags) = req.tags {
            db_dash.tags = tags.into_iter().map(Some).collect();
        }
        db_dash.updated_at = Utc::now();

        diesel::update(dashboards::table.find(dashboard_id))
            .set(&db_dash)
            .execute(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        let widgets_db: Vec<DbWidget> = dashboard_widgets::table
            .filter(dashboard_widgets::dashboard_id.eq(dashboard_id))
            .load(&mut conn)
            .unwrap_or_default();
        let filters_db: Vec<DbFilter> = dashboard_filters::table
            .filter(dashboard_filters::dashboard_id.eq(dashboard_id))
            .load(&mut conn)
            .unwrap_or_default();

        let widgets: Vec<Widget> = widgets_db.into_iter().map(db_widget_to_widget).collect();
        let filters: Vec<DashboardFilter> =
            filters_db.into_iter().map(db_filter_to_filter).collect();

        Ok::<Dashboard, DashboardsError>(db_dashboard_to_dashboard(db_dash, widgets, filters))
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}

pub async fn handle_delete_dashboard(
    State(state): State<Arc<DashboardsState>>,
    Path(dashboard_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, DashboardsError> {
    let pool = state.pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;

        let deleted = diesel::delete(dashboards::table.find(dashboard_id))
            .execute(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        if deleted == 0 {
            return Err(DashboardsError::NotFound("Dashboard not found".to_string()));
        }

        Ok::<(), DashboardsError>(())
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_get_templates(
    State(state): State<Arc<DashboardsState>>,
) -> Result<Json<Vec<Dashboard>>, DashboardsError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;
        let branch_id = get_default_bot(&mut conn);

        let db_dashboards: Vec<DbDashboard> = dashboards::table
            .filter(dashboards::branch_id.eq(&branch_id))
            .filter(dashboards::is_template.eq(true))
            .order(dashboards::created_at.desc())
            .load(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        let templates: Vec<Dashboard> = db_dashboards
            .into_iter()
            .map(|db| db_dashboard_to_dashboard(db, vec![], vec![]))
            .collect();

        Ok::<Vec<Dashboard>, DashboardsError>(templates)
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}
