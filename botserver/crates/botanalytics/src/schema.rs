// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::organizations;
// Canonical definition lives in `botschema` (#1497).
pub use botschema::users;
// Canonical definition lives in `botschema` (#1497).
pub use botschema::okr_objectives;

pub use botschema::okr_key_results;

pub use botschema::okr_checkins;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::okr_templates;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::dashboards::dashboards;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::dashboards::dashboard_widgets;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::dashboards::dashboard_data_sources;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::dashboards::dashboard_filters;

// `organizations`, `users`, `dashboard_widgets` and `dashboard_filters` are
// The `allow_tables_to_appear_in_same_query!` list is unnecessary with Diesel 2.3
// once the pairs cross crates (#1497).
