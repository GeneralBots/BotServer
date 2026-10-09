use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonType {
    PreviousPeriod,
    PreviousYear,
    Target,
    Custom,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiThresholds {
    pub good: f64,
    pub warning: f64,
    pub bad: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConfig {
    pub latitude_field: String,
    pub longitude_field: String,
    pub value_field: Option<String>,
    pub label_field: Option<String>,
    pub map_style: Option<String>,
    pub zoom: Option<i32>,
    pub center: Option<MapCenter>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapCenter {
    pub lat: f64,
    pub lng: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WidgetStyle {
    pub background_color: Option<String>,
    pub border_color: Option<String>,
    pub border_radius: Option<i32>,
    pub padding: Option<i32>,
    pub font_size: Option<i32>,
    pub text_color: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuery {
    pub source_id: Option<Uuid>,
    pub query_type: QueryType,
    pub sql: Option<String>,
    pub table: Option<String>,
    pub fields: Option<Vec<String>>,
    pub filters: Option<Vec<QueryFilter>>,
    pub group_by: Option<Vec<String>>,
    pub order_by: Option<Vec<OrderBy>>,
    pub limit: Option<i32>,
    pub aggregations: Option<Vec<Aggregation>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    Sql,
    Table,
    Api,
    Realtime,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    In,
    NotIn,
    Between,
    IsNull,
    IsNotNull,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    pub field: String,
    pub direction: SortDirection,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregation {
    pub field: String,
    pub function: AggregateFunction,
    pub alias: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AggregateFunction {
    Sum,
    Avg,
    Min,
    Max,
    Count,
    CountDistinct,
    First,
    Last,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardFilter {
    pub id: Uuid,
    pub name: String,
    pub field: String,
    pub filter_type: DashboardFilterType,
    pub default_value: Option<serde_json::Value>,
    pub options: Vec<FilterOption>,
    pub linked_widgets: Vec<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DashboardFilterType {
    Text,
    Number,
    Date,
    DateRange,
    Select,
    MultiSelect,
    Checkbox,
    Slider,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOption {
    pub value: serde_json::Value,
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceRef {
    pub id: Uuid,
    pub name: String,
    pub source_type: DataSourceType,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub source_type: DataSourceType,
    pub connection: DataSourceConnection,
    pub schema: Option<DataSourceSchema>,
    pub refresh_schedule: Option<String>,
    pub last_sync: Option<DateTime<Utc>>,
    pub status: DataSourceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    Postgresql,
    Mysql,
    Sqlserver,
    Oracle,
    Mongodb,
    Bigquery,
    Snowflake,
    Redshift,
    Elasticsearch,
    RestApi,
    GraphqlApi,
    Csv,
    Excel,
    GoogleSheets,
    Airtable,
    InternalTables,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataSourceConnection {
    pub host: Option<String>,
    pub port: Option<i32>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password_vault_key: Option<String>,
    pub ssl: Option<bool>,
    pub url: Option<String>,
    pub api_key_vault_key: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub connection_string_vault_key: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceSchema {
    pub tables: Vec<TableSchema>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceStatus {
    Active,
    Inactive,
    Error,
    Syncing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationalQuery {
    pub id: Uuid,
    pub dashboard_id: Option<Uuid>,
    pub user_id: Uuid,
    pub natural_language: String,
    pub generated_query: Option<String>,
    pub result_widget: Option<Widget>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetData {
    pub widget_id: Uuid,
    pub data: serde_json::Value,
    pub fetched_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDashboardsQuery {
    pub owner_id: Option<Uuid>,
    pub tag: Option<String>,
    pub is_template: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDashboardRequest {
    pub name: String,
    pub description: Option<String>,
    pub layout: Option<DashboardLayout>,
    pub is_public: Option<bool>,
    pub tags: Option<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDashboardRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub layout: Option<DashboardLayout>,
    pub is_public: Option<bool>,
    pub refresh_interval: Option<i32>,
    pub tags: Option<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddWidgetRequest {
    pub widget_type: WidgetType,
    pub title: String,
    pub position: WidgetPosition,
    pub config: WidgetConfig,
    pub data_query: Option<DataQuery>,
    pub style: Option<WidgetStyle>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWidgetRequest {
    pub title: Option<String>,
    pub position: Option<WidgetPosition>,
    pub config: Option<WidgetConfig>,
    pub data_query: Option<DataQuery>,
    pub style: Option<WidgetStyle>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDataSourceRequest {
    pub name: String,
    pub description: Option<String>,
    pub source_type: DataSourceType,
    pub connection: DataSourceConnection,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationalQueryRequest {
    pub query: String,
    pub data_source_id: Option<Uuid>,
    pub context: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationalQueryResponse {
    pub query: ConversationalQuery,
    pub data: Option<serde_json::Value>,
    pub suggested_visualization: Option<WidgetType>,
    pub explanation: String,
}
