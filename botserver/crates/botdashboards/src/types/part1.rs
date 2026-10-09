use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub layout: DashboardLayout,
    pub widgets: Vec<Widget>,
    pub data_sources: Vec<DataSourceRef>,
    pub filters: Vec<DashboardFilter>,
    pub refresh_interval: Option<i32>,
    pub is_public: bool,
    pub is_template: bool,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    pub columns: i32,
    pub row_height: i32,
    pub gap: i32,
    pub responsive_breakpoints: Option<ResponsiveBreakpoints>,
}
impl Default for DashboardLayout {
    fn default() -> Self {
        Self {
            columns: 12,
            row_height: 80,
            gap: 16,
            responsive_breakpoints: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsiveBreakpoints {
    pub mobile: i32,
    pub tablet: i32,
    pub desktop: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: Uuid,
    pub widget_type: WidgetType,
    pub title: String,
    pub position: WidgetPosition,
    pub config: WidgetConfig,
    pub data_query: Option<DataQuery>,
    pub style: Option<WidgetStyle>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetPosition {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    LineChart,
    BarChart,
    PieChart,
    DonutChart,
    AreaChart,
    ScatterPlot,
    Heatmap,
    Table,
    Kpi,
    Gauge,
    Map,
    Text,
    Image,
    Iframe,
    Filter,
    DateRange,
}
impl std::fmt::Display for WidgetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::LineChart => "line_chart",
            Self::BarChart => "bar_chart",
            Self::PieChart => "pie_chart",
            Self::DonutChart => "donut_chart",
            Self::AreaChart => "area_chart",
            Self::ScatterPlot => "scatter_plot",
            Self::Heatmap => "heatmap",
            Self::Table => "table",
            Self::Kpi => "kpi",
            Self::Gauge => "gauge",
            Self::Map => "map",
            Self::Text => "text",
            Self::Image => "image",
            Self::Iframe => "iframe",
            Self::Filter => "filter",
            Self::DateRange => "date_range",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for WidgetType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "line_chart" => Ok(Self::LineChart),
            "bar_chart" => Ok(Self::BarChart),
            "pie_chart" => Ok(Self::PieChart),
            "donut_chart" => Ok(Self::DonutChart),
            "area_chart" => Ok(Self::AreaChart),
            "scatter_plot" => Ok(Self::ScatterPlot),
            "heatmap" => Ok(Self::Heatmap),
            "table" => Ok(Self::Table),
            "kpi" => Ok(Self::Kpi),
            "gauge" => Ok(Self::Gauge),
            "map" => Ok(Self::Map),
            "text" => Ok(Self::Text),
            "image" => Ok(Self::Image),
            "iframe" => Ok(Self::Iframe),
            "filter" => Ok(Self::Filter),
            "date_range" => Ok(Self::DateRange),
            _ => Err(format!("Unknown widget type: {s}")),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WidgetConfig {
    pub chart_config: Option<ChartConfig>,
    pub table_config: Option<TableConfig>,
    pub kpi_config: Option<KpiConfig>,
    pub map_config: Option<MapConfig>,
    pub text_content: Option<String>,
    pub image_url: Option<String>,
    pub iframe_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    pub x_axis: Option<String>,
    pub y_axis: Option<String>,
    pub series: Vec<ChartSeries>,
    pub legend_position: Option<String>,
    pub show_labels: Option<bool>,
    pub stacked: Option<bool>,
    pub colors: Option<Vec<String>>,
    pub animations: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSeries {
    pub name: String,
    pub field: String,
    pub color: Option<String>,
    pub series_type: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfig {
    pub columns: Vec<TableColumn>,
    pub page_size: Option<i32>,
    pub sortable: Option<bool>,
    pub filterable: Option<bool>,
    pub export_enabled: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub field: String,
    pub header: String,
    pub width: Option<i32>,
    pub format: Option<ColumnFormat>,
    pub sortable: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColumnFormat {
    Text,
    Number,
    Currency,
    Percentage,
    Date,
    DateTime,
    Boolean,
    Link,
    Image,
    Progress,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiConfig {
    pub value_field: String,
    pub comparison_field: Option<String>,
    pub comparison_type: Option<ComparisonType>,
    pub format: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub thresholds: Option<KpiThresholds>,
}
impl std::fmt::Display for DashboardFilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Date => "date",
            Self::DateRange => "date_range",
            Self::Select => "select",
            Self::MultiSelect => "multi_select",
            Self::Checkbox => "checkbox",
            Self::Slider => "slider",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for DashboardFilterType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "date" => Ok(Self::Date),
            "date_range" => Ok(Self::DateRange),
            "select" => Ok(Self::Select),
            "multi_select" => Ok(Self::MultiSelect),
            "checkbox" => Ok(Self::Checkbox),
            "slider" => Ok(Self::Slider),
            _ => Err(format!("Unknown filter type: {s}")),
        }
    }
}
impl std::fmt::Display for DataSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Postgresql => "postgresql",
            Self::Mysql => "mysql",
            Self::Sqlserver => "sqlserver",
            Self::Oracle => "oracle",
            Self::Mongodb => "mongodb",
            Self::Bigquery => "bigquery",
            Self::Snowflake => "snowflake",
            Self::Redshift => "redshift",
            Self::Elasticsearch => "elasticsearch",
            Self::RestApi => "rest_api",
            Self::GraphqlApi => "graphql_api",
            Self::Csv => "csv",
            Self::Excel => "excel",
            Self::GoogleSheets => "google_sheets",
            Self::Airtable => "airtable",
            Self::InternalTables => "internal_tables",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for DataSourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "postgresql" => Ok(Self::Postgresql),
            "mysql" => Ok(Self::Mysql),
            "sqlserver" => Ok(Self::Sqlserver),
            "oracle" => Ok(Self::Oracle),
            "mongodb" => Ok(Self::Mongodb),
            "bigquery" => Ok(Self::Bigquery),
            "snowflake" => Ok(Self::Snowflake),
            "redshift" => Ok(Self::Redshift),
            "elasticsearch" => Ok(Self::Elasticsearch),
            "rest_api" => Ok(Self::RestApi),
            "graphql_api" => Ok(Self::GraphqlApi),
            "csv" => Ok(Self::Csv),
            "excel" => Ok(Self::Excel),
            "google_sheets" => Ok(Self::GoogleSheets),
            "airtable" => Ok(Self::Airtable),
            "internal_tables" => Ok(Self::InternalTables),
            _ => Err(format!("Unknown data source type: {s}")),
        }
    }
}
impl std::fmt::Display for DataSourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Error => "error",
            Self::Syncing => "syncing",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for DataSourceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "error" => Ok(Self::Error),
            "syncing" => Ok(Self::Syncing),
            _ => Err(format!("Unknown status: {s}")),
        }
    }
}
