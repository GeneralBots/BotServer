// Diesel table definitions for dashboards (migrations 6.0.19 + 6.5.23).

// Canonical definition lives in `botschema` (#1497).
pub use botschema::dashboards::dashboards;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::dashboard_data_sources;

// Canonical definition lives in `botschema::dashboards` (#1497).
pub use botschema::dashboards::dashboard_widgets;

// Canonical definition lives in `botschema::dashboards` (#1497).
pub use botschema::dashboards::dashboard_filters;

// `botschema` does not re-export the `dashboards` module at the root (its own
// `dashboards` table would collide with the module name), so the canonical path
// goes through the module.
pub use botschema::dashboards::conversational_queries;

// `dashboard_widgets`, `dashboard_filters` and the `allow_tables_to_appear_in_same_query!`
// list are re-exported from `botschema::dashboards`; their join rules live there (#1497).
