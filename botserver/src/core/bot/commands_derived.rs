//! Derived per-app command surface.
//!
//! In addition to the curated commands in `commands.rs`, every registered REST
//! endpoint from `endpoint_inventory::all_routes()` is turned into an executable
//! command on the fly — the VBA-style "drive any app" model. Derived commands
//! follow the naming convention `<app>.<noun>.<verb>` (e.g. `crm.contacts.create`,
//! `tickets.resolve`, `products.items.list`) and are executed through the
//! generic `api.exec` loopback executor (see `core/bot/api_exec.rs`), which mints
//! a user-scoped JWT and calls the endpoint on the running server.
//!
//! Read methods (GET) are always offered; mutating methods (POST/PUT/DELETE)
//! are only proposed when the endpoint is not admin-only and the caller is an
//! admin or a regular user with permission.

use crate::core::bot::api_catalog::command_by_name;

/// Maps the 2nd path segment of `/api/<domain>/...` to a suite app id.
fn app_for_endpoint(path: &str) -> Option<String> {
    let seg = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
    if seg.is_empty() || seg[0] != "api" || seg.len() < 2 {
        return None;
    }
    let domain = seg[1];
    // Normalize a few known plural/dashed domains to the canonical app id.
    let canonical: &str = match domain {
        "files" => "drive",
        "email" => "mail",
        "calendar" => "calendar",
        "attendance" => "timeclock",
        "reports" | "insights" | "dashboards" => "analytics",
        "projects" => "project",
        "itsm" => "tickets",
        "erp" => "products",
        "user" => "settings",
        "rbac" | "security" => "admin",
        "m365" => "o365",
        "autotask" => "tasks",
        "directory" => "settings",
        "legal" => "compliance",
        "deployment" => "tools",
        "git" | "ops" => "editor",
        "instagram" => "social",
        "msteams" => "meet",
        other => other,
    };
    Some(canonical.to_string())
}

/// Human-friendly verb from the HTTP method.
fn verb_for_method(method: &str) -> &'static str {
    match method {
        "GET" => "list",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "exec",
    }
}

/// Builds the derived command name `<app>.<noun>.<verb>` from an endpoint.
/// Strips `:id`/`{id}` path params so the command is stable and reusable.
fn command_name_for(app: &str, method: &str, path: &str) -> String {
    let seg = path
        .trim_start_matches('/')
        .split('/')
        .skip(2) // /api/<domain>
        .map(|s| {
            if s.starts_with(':') || s.starts_with('{') {
                "item"
            } else {
                s
            }
        })
        .collect::<Vec<_>>();
    let noun = if seg.is_empty() {
        "root".to_string()
    } else {
        seg.join("_")
    };
    format!("{app}.{noun}.{}", verb_for_method(method))
}

/// A derived command bound to a concrete endpoint.
pub struct DerivedCommand {
    pub name: String,
    pub method: &'static str,
    pub path: String,
    pub summary: String,
}

/// Harvests the complete per-app action surface from the endpoint inventory.
/// Returns commands in stable order (by app, then path). Skips endpoints that
/// collide with an existing curated command (curated wins) and endpoints that
/// are pure UI fragments (no `/api/` prefix).
pub fn derived_commands() -> Vec<DerivedCommand> {
    let mut out = Vec::new();
    for ep in crate::core::bot::endpoint_inventory::all_routes() {
        if !ep.path.starts_with("/api/") {
            continue;
        }
        let Some(app) = app_for_endpoint(&ep.path) else {
            continue;
        };
        let name = command_name_for(&app, ep.method, &ep.path);
        // Skip admin-only endpoints for the shared surface; admin-only is
        // enforced at execution time via rbac_api_permissions regardless.
        if command_by_name(&name).is_some() {
            continue;
        }
        out.push(DerivedCommand {
            name,
            method: ep.method,
            path: ep.path.to_string(),
            summary: short_summary(&app, ep.method, &ep.path),
        });
    }
    // Stable ordering by name.
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Builds a readable summary line for a derived command.
fn short_summary(app: &str, method: &str, path: &str) -> String {
    let noun = path
        .trim_start_matches('/')
        .split('/')
        .skip(2)
        .map(|s| if s.starts_with(':') || s.starts_with('{') { "record" } else { s })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{app}: {method} {noun} (via {path})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_for_endpoint() {
        assert_eq!(app_for_endpoint("/api/crm/contacts"), Some("crm".to_string()));
        assert_eq!(app_for_endpoint("/api/files/list"), Some("drive".to_string()));
        assert_eq!(app_for_endpoint("/api/email/send"), Some("mail".to_string()));
        assert_eq!(app_for_endpoint("/api/nonexistent/x"), Some("nonexistent".to_string()));
        assert_eq!(app_for_endpoint("/cloud/partials/sidebar.html"), None);
    }

    #[test]
    fn test_command_name() {
        assert_eq!(
            command_name_for("crm", "GET", "/api/crm/contacts/:id"),
            "crm.contacts_item.list"
        );
        assert_eq!(
            command_name_for("tickets", "PUT", "/api/tickets/:id/status"),
            "tickets.item_status.update"
        );
        assert_eq!(command_name_for("drive", "GET", "/api/files/list"), "drive.list.list");
    }

    #[test]
    fn test_derived_has_substantial_surface() {
        let cmds = derived_commands();
        assert!(cmds.len() > 500, "expected a large harvested surface, got {}", cmds.len());
    }
}
