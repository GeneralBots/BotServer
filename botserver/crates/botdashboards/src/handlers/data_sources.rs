use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::DashboardsError;
use crate::schema::{conversational_queries, dashboard_data_sources};
use crate::storage::{db_data_source_to_data_source, DbConversationalQuery, DbDataSource};
use crate::types::{
    ConversationalQuery, ConversationalQueryRequest, ConversationalQueryResponse,
    CreateDataSourceRequest, DataSource, WidgetType,
};
use crate::DashboardsState;

pub async fn handle_list_data_sources(
    State(state): State<Arc<DashboardsState>>,
) -> Result<Json<Vec<DataSource>>, DashboardsError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;
        let branch_id = get_default_bot(&mut conn);

        let db_sources: Vec<DbDataSource> = dashboard_data_sources::table
            .filter(dashboard_data_sources::branch_id.eq(branch_id))
            .order(dashboard_data_sources::created_at.desc())
            .load(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        let sources: Vec<DataSource> = db_sources
            .into_iter()
            .map(db_data_source_to_data_source)
            .collect();
        Ok::<Vec<DataSource>, DashboardsError>(sources)
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}

pub async fn handle_create_data_source(
    State(state): State<Arc<DashboardsState>>,
    Json(req): Json<CreateDataSourceRequest>,
) -> Result<Json<DataSource>, DashboardsError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;
        let branch_id = get_default_bot(&mut conn);
        let now = Utc::now();

        let db_source = DbDataSource {
            id: Uuid::new_v4(),
            org_id: Uuid::nil(),
            bot_id: Uuid::nil(),
            name: req.name,
            description: req.description,
            source_type: req.source_type.to_string(),
            connection: serde_json::to_value(&req.connection).unwrap_or_default(),
            schema_definition: serde_json::json!({}),
            refresh_schedule: None,
            last_sync: None,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
            branch_id,
        };

        diesel::insert_into(dashboard_data_sources::table)
            .values(&db_source)
            .execute(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        Ok::<DataSource, DashboardsError>(db_data_source_to_data_source(db_source))
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}

fn host_port_from_url(url: &str, default_port: u16) -> Option<(String, u16)> {
    let rest = url.split("://").nth(1)?;
    let authority = rest.split(['/', '?']).next()?;
    let authority = authority.rsplit('@').next()?;
    match authority.rsplit_once(':') {
        Some((host, port)) => Some((host.to_string(), port.parse().unwrap_or(default_port))),
        None => Some((authority.to_string(), default_port)),
    }
}

fn connection_endpoint(config: &serde_json::Value) -> Option<(String, u16)> {
    let connection = config.get("connection").filter(|c| c.is_object()).unwrap_or(config);
    if let Some(url) = connection.get("url").and_then(|v| v.as_str()) {
        return host_port_from_url(url, 80);
    }
    let host = connection.get("host").and_then(|v| v.as_str())?;
    let port = connection.get("port").and_then(|v| v.as_i64()).unwrap_or(80);
    Some((host.to_string(), port as u16))
}

fn probe_connection(host: &str, port: u16) -> Result<(), String> {
    use std::net::{TcpStream, ToSocketAddrs};
    let addr = format!("{host}:{port}");
    let timeout = std::time::Duration::from_secs(3);
    let addrs = addr
        .to_socket_addrs()
        .map_err(|e| format!("Could not resolve {addr}: {e}"))?;
    for a in addrs {
        if TcpStream::connect_timeout(&a, timeout).is_ok() {
            return Ok(());
        }
    }
    Err(format!("Could not connect to {addr}"))
}

pub async fn handle_test_data_source(
    State(state): State<Arc<DashboardsState>>,
    Path(source_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, DashboardsError> {
    let pool = state.pool.clone();

    let config = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;

        #[derive(diesel::QueryableByName)]
        struct ConfigRow {
            #[diesel(sql_type = diesel::sql_types::Jsonb)]
            connection: serde_json::Value,
        }
        let row: ConfigRow = diesel::sql_query(
            "SELECT connection FROM dashboard_data_sources WHERE id = $1",
        )
        .bind::<diesel::sql_types::Uuid, _>(source_id)
        .get_result(&mut conn)
        .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        Ok::<Option<serde_json::Value>, DashboardsError>(Some(row.connection))
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    let Some(config) = config else {
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": "Data source not found"
        })));
    };

    match connection_endpoint(&config) {
        Some((host, port)) => match probe_connection(&host, port) {
            Ok(()) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "Connection test successful"
            }))),
            Err(e) => Ok(Json(serde_json::json!({ "success": false, "message": e }))),
        },
        None => Ok(Json(serde_json::json!({
            "success": false,
            "message": "No host/port/url in connection config"
        }))),
    }
}

pub async fn handle_test_data_source_no_id(
    State(_state): State<Arc<DashboardsState>>,
    Json(config): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, DashboardsError> {
    match connection_endpoint(&config) {
        Some((host, port)) => match probe_connection(&host, port) {
            Ok(()) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "Connection test successful"
            }))),
            Err(e) => Ok(Json(serde_json::json!({ "success": false, "message": e }))),
        },
        None => Ok(Json(serde_json::json!({
            "success": false,
            "message": "No host/port/url in connection config"
        }))),
    }
}

pub async fn handle_delete_data_source(
    State(state): State<Arc<DashboardsState>>,
    Path(source_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, DashboardsError> {
    let pool = state.pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;

        diesel::delete(dashboard_data_sources::table.find(source_id))
            .execute(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        Ok::<(), DashboardsError>(())
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(serde_json::json!({ "success": true })))
}

fn analyze_query_intent(query: &str) -> (WidgetType, String) {
    let query_lower = query.to_lowercase();

    if query_lower.contains("trend")
        || query_lower.contains("over time")
        || query_lower.contains("timeline")
    {
        (
            WidgetType::LineChart,
            "Showing data as a line chart to visualize trends over time".to_string(),
        )
    } else if query_lower.contains("compare")
        || query_lower.contains("by category")
        || query_lower.contains("breakdown")
    {
        (
            WidgetType::BarChart,
            "Using a bar chart to compare values across categories".to_string(),
        )
    } else if query_lower.contains("distribution")
        || query_lower.contains("percentage")
        || query_lower.contains("share")
    {
        (
            WidgetType::PieChart,
            "Displaying distribution as a pie chart".to_string(),
        )
    } else if query_lower.contains("total")
        || query_lower.contains("count")
        || query_lower.contains("sum")
        || query_lower.contains("kpi")
    {
        (
            WidgetType::Kpi,
            "Showing as a KPI card for quick insight".to_string(),
        )
    } else if query_lower.contains("table")
        || query_lower.contains("list")
        || query_lower.contains("details")
    {
        (
            WidgetType::Table,
            "Presenting data in a table format for detailed view".to_string(),
        )
    } else if query_lower.contains("map")
        || query_lower.contains("location")
        || query_lower.contains("geographic")
    {
        (
            WidgetType::Map,
            "Visualizing geographic data on a map".to_string(),
        )
    } else if query_lower.contains("gauge")
        || query_lower.contains("progress")
        || query_lower.contains("target")
    {
        (
            WidgetType::Gauge,
            "Showing progress toward a target as a gauge".to_string(),
        )
    } else {
        (
            WidgetType::BarChart,
            "Defaulting to bar chart for general visualization".to_string(),
        )
    }
}

pub async fn handle_conversational_query(
    State(state): State<Arc<DashboardsState>>,
    Json(req): Json<ConversationalQueryRequest>,
) -> Result<Json<ConversationalQueryResponse>, DashboardsError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;
    let query_text = req.query.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| DashboardsError::Database(e.to_string()))?;
        let branch_id = get_default_bot(&mut conn);
        let now = Utc::now();

        let db_query = DbConversationalQuery {
            id: Uuid::new_v4(),
            org_id: Uuid::nil(),
            bot_id: Uuid::nil(),
            dashboard_id: None,
            user_id: Uuid::nil(),
            natural_language: query_text.clone(),
            generated_query: None,
            result_widget_config: None,
            created_at: now,
            branch_id,
        };

        diesel::insert_into(conversational_queries::table)
            .values(&db_query)
            .execute(&mut conn)
            .map_err(|e: diesel::result::Error| DashboardsError::Database(e.to_string()))?;

        let (suggested_viz, explanation) = analyze_query_intent(&query_text);

        let conv_query = ConversationalQuery {
            id: db_query.id,
            dashboard_id: None,
            user_id: db_query.user_id,
            natural_language: db_query.natural_language,
            generated_query: None,
            result_widget: None,
            created_at: db_query.created_at,
        };

        Ok::<ConversationalQueryResponse, DashboardsError>(ConversationalQueryResponse {
            query: conv_query,
            data: Some(serde_json::json!([])),
            suggested_visualization: Some(suggested_viz),
            explanation,
        })
    })
    .await
    .map_err(|e: tokio::task::JoinError| DashboardsError::Internal(e.to_string()))??;

    Ok(Json(result))
}
