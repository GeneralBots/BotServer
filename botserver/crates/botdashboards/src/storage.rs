use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{
    conversational_queries, dashboard_data_sources, dashboard_filters, dashboard_widgets,
    dashboards,
};

use super::types::{
    Dashboard, DashboardFilter, DashboardFilterType, DashboardLayout, DataSource,
    DataSourceConnection, DataSourceSchema, DataSourceStatus, DataSourceType, DataQuery,
    FilterOption, Widget, WidgetConfig, WidgetPosition, WidgetStyle, WidgetType,
};

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = dashboards)]
pub struct DbDashboard {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub layout: serde_json::Value,
    pub refresh_interval: Option<i32>,
    pub is_public: bool,
    pub is_template: bool,
    pub tags: Vec<Option<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = dashboard_widgets)]
pub struct DbWidget {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub widget_type: String,
    pub title: String,
    pub position_x: i32,
    pub position_y: i32,
    pub width: i32,
    pub height: i32,
    pub config: serde_json::Value,
    pub data_query: Option<serde_json::Value>,
    pub style: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = dashboard_data_sources)]
pub struct DbDataSource {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub source_type: String,
    pub connection: serde_json::Value,
    pub schema_definition: serde_json::Value,
    pub refresh_schedule: Option<String>,
    pub last_sync: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = dashboard_filters)]
pub struct DbFilter {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub name: String,
    pub field: String,
    pub filter_type: String,
    pub default_value: Option<serde_json::Value>,
    pub options: serde_json::Value,
    pub linked_widgets: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = conversational_queries)]
pub struct DbConversationalQuery {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub dashboard_id: Option<Uuid>,
    pub user_id: Uuid,
    pub natural_language: String,
    pub generated_query: Option<String>,
    pub result_widget_config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

pub fn db_dashboard_to_dashboard(
    db: DbDashboard,
    widgets: Vec<Widget>,
    filters: Vec<DashboardFilter>,
) -> Dashboard {
    let layout: DashboardLayout = serde_json::from_value(db.layout).unwrap_or_default();
    let tags: Vec<String> = db.tags.into_iter().flatten().collect();

    Dashboard {
        id: db.id,
        organization_id: db.branch_id,
        owner_id: db.owner_id,
        name: db.name,
        description: db.description,
        layout,
        widgets,
        data_sources: vec![],
        filters,
        refresh_interval: db.refresh_interval,
        is_public: db.is_public,
        is_template: db.is_template,
        tags,
        created_at: db.created_at,
        updated_at: db.updated_at,
    }
}

pub fn db_widget_to_widget(db: DbWidget) -> Widget {
    let widget_type: WidgetType = db.widget_type.parse().unwrap_or(WidgetType::Text);
    let config: WidgetConfig = serde_json::from_value(db.config).unwrap_or_default();
    let data_query: Option<DataQuery> = db.data_query.and_then(|v| serde_json::from_value(v).ok());
    let style: Option<WidgetStyle> = serde_json::from_value(db.style).ok();

    Widget {
        id: db.id,
        widget_type,
        title: db.title,
        position: WidgetPosition {
            x: db.position_x,
            y: db.position_y,
            width: db.width,
            height: db.height,
        },
        config,
        data_query,
        style,
    }
}

pub fn db_filter_to_filter(db: DbFilter) -> DashboardFilter {
    let filter_type: DashboardFilterType = db
        .filter_type
        .parse()
        .unwrap_or(DashboardFilterType::Text);
    let options: Vec<FilterOption> = serde_json::from_value(db.options).unwrap_or_default();
    let linked_widgets: Vec<Uuid> = serde_json::from_value(db.linked_widgets).unwrap_or_default();

    DashboardFilter {
        id: db.id,
        name: db.name,
        field: db.field,
        filter_type,
        default_value: db.default_value,
        options,
        linked_widgets,
    }
}

pub fn db_data_source_to_data_source(db: DbDataSource) -> DataSource {
    let source_type: DataSourceType = db
        .source_type
        .parse()
        .unwrap_or(DataSourceType::InternalTables);
    let connection: DataSourceConnection =
        serde_json::from_value(db.connection.clone()).unwrap_or_default();
    let schema: Option<DataSourceSchema> = serde_json::from_value(db.schema_definition).ok();
    let status: DataSourceStatus = db.status.parse().unwrap_or(DataSourceStatus::Inactive);

    DataSource {
        id: db.id,
        organization_id: db.branch_id,
        name: db.name,
        description: db.description,
        source_type,
        connection,
        schema,
        refresh_schedule: db.refresh_schedule,
        last_sync: db.last_sync,
        status,
        created_at: db.created_at,
        updated_at: db.updated_at,
    }
}
