use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::DashboardsError;
use crate::schema::dashboard_widgets;
use crate::storage::{db_widget_to_widget, DbWidget};
use crate::types::{AddWidgetRequest, UpdateWidgetRequest, Widget, WidgetData};
use crate::DashboardsState;

pub async fn handle_add_widget(
    State(state): State<Arc<DashboardsState>>,
    Path(dashboard_id): Path<Uuid>,
    Json(req): Json<AddWidgetRequest>,
) -> Result<Json<Widget>, DashboardsError> {
    let pool = state.pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;
        let now = Utc::now();

        let db_widget = DbWidget {
            id: Uuid::new_v4(),
            dashboard_id,
            widget_type: req.widget_type.to_string(),
            title: req.title,
            position_x: req.position.x,
            position_y: req.position.y,
            width: req.position.width,
            height: req.position.height,
            config: serde_json::to_value(&req.config).unwrap_or_default(),
            data_query: req.data_query.and_then(|q| serde_json::to_value(&q).ok()),
            style: serde_json::to_value(req.style.unwrap_or_default()).unwrap_or_default(),
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(dashboard_widgets::table)
            .values(&db_widget)
            .execute(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        Ok::<_, DashboardsError>(db_widget_to_widget(db_widget))
    })
    .await
    .map_err(|e| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}

pub async fn handle_update_widget(
    State(state): State<Arc<DashboardsState>>,
    Path((dashboard_id, widget_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateWidgetRequest>,
) -> Result<Json<Widget>, DashboardsError> {
    let pool = state.pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;

        let mut db_widget: DbWidget = dashboard_widgets::table
            .filter(dashboard_widgets::id.eq(widget_id))
            .filter(dashboard_widgets::dashboard_id.eq(dashboard_id))
            .first(&mut conn)
            .map_err(|_| DashboardsError::NotFound("Widget not found".to_string()))?;

        if let Some(title) = req.title {
            db_widget.title = title;
        }
        if let Some(position) = req.position {
            db_widget.position_x = position.x;
            db_widget.position_y = position.y;
            db_widget.width = position.width;
            db_widget.height = position.height;
        }
        if let Some(config) = req.config {
            db_widget.config = serde_json::to_value(&config).unwrap_or_default();
        }
        if let Some(data_query) = req.data_query {
            db_widget.data_query = serde_json::to_value(&data_query).ok();
        }
        if let Some(style) = req.style {
            db_widget.style = serde_json::to_value(&style).unwrap_or_default();
        }
        db_widget.updated_at = Utc::now();

        diesel::update(dashboard_widgets::table.find(widget_id))
            .set(&db_widget)
            .execute(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        Ok::<_, DashboardsError>(db_widget_to_widget(db_widget))
    })
    .await
    .map_err(|e| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}

pub async fn handle_delete_widget(
    State(state): State<Arc<DashboardsState>>,
    Path((dashboard_id, widget_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, DashboardsError> {
    let pool = state.pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;

        let deleted = diesel::delete(
            dashboard_widgets::table
                .filter(dashboard_widgets::id.eq(widget_id))
                .filter(dashboard_widgets::dashboard_id.eq(dashboard_id)),
        )
        .execute(&mut conn)
        .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        if deleted == 0 {
            return Err(DashboardsError::NotFound("Widget not found".to_string()));
        }

        Ok::<_, DashboardsError>(())
    })
    .await
    .map_err(|e| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_get_widget_data(
    State(_state): State<Arc<DashboardsState>>,
    Path((_dashboard_id, widget_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<WidgetData>, DashboardsError> {
    Ok(Json(WidgetData {
        widget_id,
        data: serde_json::json!([]),
        fetched_at: Utc::now(),
    }))
}
