// The five dashboard tables are declared in `dashboards.rs`; re-export them so
// the `analytics` feature keeps their paths and a single join rule set (#1497).
pub use crate::dashboards::{conversational_queries, dashboard_data_sources, dashboard_filters, dashboard_widgets, dashboards};










diesel::table! {
    dashboard_widget_data_sources (id) {
        id -> Uuid,
        widget_id -> Uuid,
        data_source_id -> Uuid,
        created_at -> Timestamptz,
    }
}



diesel::joinable!(dashboard_widget_data_sources -> dashboard_widgets (widget_id));
diesel::joinable!(dashboard_widget_data_sources -> dashboard_data_sources (data_source_id));
