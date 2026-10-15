//! Enterprise-grade application catalog.
//!
//! Single source of truth for every application in the suite. The frontend
//! launchers (start menu, apps menu) consume this catalog through
//! `GET /api/apps/catalog` instead of maintaining their own duplicate lists.

use serde::Serialize;

/// Launch surface of an application. Widgets additionally declare a
/// [`WidgetSpec`] so the desktop can render them as live tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AppKind {
    /// Regular app opened as a desktop window (#1160).
    App,
    /// Always-visible desktop tile; also windowable and isolable.
    Widget,
}

/// Compact render hints for widget-capable apps (#1160).
#[derive(Debug, Clone, Serialize)]
pub struct WidgetSpec {
    /// Full web page rendered in a sandboxed iframe (cross-origin safe).
    pub url: Option<String>,
    /// Minimum grid cells, e.g. (2, 2).
    pub size_w: u16,
    pub size_h: u16,
    /// Optional poll interval in seconds; `None` means static.
    pub refresh_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppDefinition {
    pub id: String,
    pub title: String,
    pub category: String,
    pub color: String,
    pub url: String,
    pub description: String,
    pub keywords: String,
    pub icon: String,
    /// Whether this app is a windowable app or a desktop widget.
    pub kind: AppKind,
    /// Widget render spec for `kind == Widget` (and for apps that expose one).
    pub widget: Option<WidgetSpec>,
    /// Vibe-published apps may opt in to auto-pin on the launcher (#1160).
    pub launcher_default: bool,
}

pub const CATEGORIES: &[(&str, &str)] = &[
    ("ai", "AI & Assistants"),
    ("business", "Business"),
    ("office", "Office & Productivity"),
    ("dev", "Development"),
    ("system", "System & Tools"),
];

pub fn category_labels() -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, label) in CATEGORIES {
        map.insert((*key).to_string(), serde_json::json!(label));
    }
    serde_json::Value::Object(map)
}
/// Identity and presentation of one catalog entry. Grouped so the `app` and
/// `widget_app` constructors keep short argument lists.
#[derive(Debug, Clone, Copy)]
pub struct AppSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub color: &'static str,
    pub url: &'static str,
    pub description: &'static str,
    pub keywords: &'static str,
    pub icon: &'static str,
}

fn definition(spec: AppSpec, kind: AppKind, widget: Option<WidgetSpec>) -> AppDefinition {
    let AppSpec {
        id,
        title,
        category,
        color,
        url,
        description,
        keywords,
        icon,
    } = spec;
    AppDefinition {
        id: id.to_string(),
        title: title.to_string(),
        category: category.to_string(),
        color: color.to_string(),
        url: url.to_string(),
        description: description.to_string(),
        keywords: keywords.to_string(),
        icon: icon.to_string(),
        kind,
        widget,
        launcher_default: false,
    }
}

/// A regular application entry.
fn app(spec: AppSpec) -> AppDefinition {
    definition(spec, AppKind::App, None)
}

/// A widget-capable entry: rendered as a live desktop tile through `spec`.
fn widget_app(spec: AppSpec, widget: WidgetSpec) -> AppDefinition {
    definition(spec, AppKind::Widget, Some(widget))
}

/// Marks a published app to auto-pin on the launcher when the frontend first
/// sees it (Vibe `publish` with `launcher: true`, #1160).
pub fn with_launcher_default(mut definition: AppDefinition) -> AppDefinition {
    definition.launcher_default = true;
    definition
}

pub fn all_apps() -> Vec<AppDefinition> {
    let mut apps = part1::entries();
    apps.extend(part2::entries());
    apps
}

mod part1;
mod part2;
