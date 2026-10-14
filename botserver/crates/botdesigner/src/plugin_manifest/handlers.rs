use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::{get, post, delete},
    Router,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::DesignerState;
use super::{
    ExposedFunction,
    load_manifests_from_drive, html_escape, DEFAULT_PLUGINS_PREFIX,
};

pub async fn handle_list_plugins(
    State(state): State<Arc<DesignerState>>,
) -> impl IntoResponse {
    let bucket = &state.bucket_name;
    let plugins = load_manifests_from_drive(bucket, &state.load_from_drive);

    let plugins = match plugins {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to list plugins: {}", e);
            Vec::new()
        }
    };

    let mut html = String::new();
    html.push_str("<div id=\"plugin-list\">");

    for entry in &plugins {
        let m = &entry.manifest;
        let status_class = if m.enabled { "active" } else { "inactive" };
        let status_label = if m.enabled { "Enabled" } else { "Disabled" };
        let auth_label = m.auth_type.as_str();
        let func_count = m.functions.len();
        let funcs_html = render_functions(&m.functions);

        html.push_str(&format!(
            concat!(
                r##"<div class="plugin-card {status}">"##,
                r##"<div class="plugin-header">"##,
                r##"<div class="plugin-info">"##,
                r##"<h3 class="plugin-name">{name}</h3>"##,
                r##"<span class="plugin-version">v{version}</span>"##,
                r##"</div>"##,
                r##"<div class="plugin-status {status_class}">{status_label}</div>"##,
                r##"</div>"##,
                r##"<p class="plugin-desc">{desc}</p>"##,
                r##"<div class="plugin-meta">"##,
                r##"<span class="meta-item">Auth: {auth}</span>"##,
                r##"<span class="meta-item">Functions: {funcs}</span>"##,
                r##"<span class="meta-item">Permissions: {perms}</span>"##,
                r##"</div>"##,
                "{funcs_html}",
                r##"<div class="plugin-actions">"##,
                r##"<button class="btn btn-sm btn-outline""##,
                r##" hx-post="/api/ui/plugins/{id}/toggle""##,
                r##" hx-target="#plugin-list""##,
                r##" hx-swap="outerHTML">{toggle_label}</button>"##,
                r##"<button class="btn btn-sm btn-danger""##,
                r##" hx-delete="/api/ui/plugins/{id}""##,
                r##" hx-target="#plugin-list""##,
                r##" hx-swap="outerHTML""##,
                r##" hx-confirm="Remove this plugin?">Remove</button>"##,
                r##"</div>"##,
                r##"</div>"##,
            ),
            status = status_class,
            name = html_escape(&m.name),
            version = html_escape(&m.version),
            status_class = status_class,
            status_label = status_label,
            desc = html_escape(&m.description),
            auth = auth_label,
            funcs = func_count,
            perms = m.permissions.len(),
            funcs_html = funcs_html,
            id = m.id,
            toggle_label = if m.enabled { "Disable" } else { "Enable" },
        ));
    }

    if plugins.is_empty() {
        html.push_str(r##"<div class="empty-state">"##);
        html.push_str(r##"<h3>No plugins installed</h3>"##);
        html.push_str(r##"<p>Upload a plugin manifest to get started</p>"##);
        html.push_str(r##"</div>"##);
    }

    html.push_str("</div>");
    Html(html)
}

fn render_functions(functions: &[ExposedFunction]) -> String {
    if functions.is_empty() {
        return String::new();
    }

    let mut html = String::from(
        r##"<div class="plugin-functions"><h4>Exposed Functions</h4>"##,
    );

    for func in functions {
        let params_str = func
            .params
            .iter()
            .map(|p| {
                let req = if p.required { "required" } else { "optional" };
                format!("{}:{} ({})", html_escape(&p.name), html_escape(&p.param_type), req)
            })
            .collect::<Vec<_>>()
            .join(", ");

        html.push_str(r##"<div class="func-item">"##);
        html.push_str(&format!(
            r##"<span class="func-name">{name}</span>"##,
            name = html_escape(&func.name),
        ));
        html.push_str(&format!(
            r##"<span class="func-desc">{desc}</span>"##,
            desc = html_escape(&func.description),
        ));
        if !params_str.is_empty() {
            html.push_str(&format!(
                r##"<span class="func-params">({params})</span>"##,
                params = html_escape(&params_str),
            ));
        }
        html.push_str("</div>");
    }

    html.push_str("</div>");
    html
}

pub async fn handle_toggle_plugin(
    State(state): State<Arc<DesignerState>>,
    Path(plugin_id): Path<Uuid>,
) -> impl IntoResponse {
    let bucket = &state.bucket_name;
    let plugins = match load_manifests_from_drive(bucket, &state.load_from_drive) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to load plugins for toggle: {}", e);
            return handle_list_plugins(State(state)).await;
        }
    };

    for entry in &plugins {
        if entry.manifest.id == plugin_id {
            let new_enabled = !entry.manifest.enabled;
            let manifest_path = format!("{}/{}.json", DEFAULT_PLUGINS_PREFIX, entry.manifest.name);
            let mut updated = entry.manifest.clone();
            updated.enabled = new_enabled;

            match serde_json::to_string_pretty(&updated) {
                Ok(json_str) => {
                    if let Err(e) = (state.write_to_drive)(bucket, &manifest_path, json_str.as_bytes(), "application/json") {
                        log::error!("Failed to save plugin toggle state: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("Failed to serialize plugin manifest: {}", e);
                }
            }
            break;
        }
    }

    handle_list_plugins(State(state)).await
}

pub async fn handle_remove_plugin(
    State(state): State<Arc<DesignerState>>,
    Path(plugin_id): Path<Uuid>,
) -> impl IntoResponse {
    let bucket = &state.bucket_name;
    let plugins = match load_manifests_from_drive(bucket, &state.load_from_drive) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to load plugins for removal: {}", e);
            return handle_list_plugins(State(state)).await;
        }
    };

    for entry in &plugins {
        if entry.manifest.id == plugin_id {
            let manifest_path = format!("{}/{}.json", DEFAULT_PLUGINS_PREFIX, entry.manifest.name);
            let _ = (state.write_to_drive)(bucket, &manifest_path, b"{}", "application/json");
            log::info!("Removed plugin: {} from bucket {}", entry.manifest.name, bucket);
            break;
        }
    }

    handle_list_plugins(State(state)).await
}

pub async fn handle_upload_manifest(
    State(_state): State<Arc<DesignerState>>,
) -> impl IntoResponse {
    Html(
        r##"<div class="alert alert-warning">Upload via multipart not yet implemented. Place .json files directly in the plugins/ directory of the bot bucket.</div>"##,
    )
}

pub async fn handle_plugins_page(
    State(_state): State<Arc<DesignerState>>,
) -> Html<String> {
    let html = String::from(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width,initial-scale=1.0">
    <title>Plugin Manager</title>
    <script src="/suite/js/vendor/htmx.min.js"></script>
    <style>
        *{box-sizing:border-box;margin:0;padding:0}
        body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#f5f5f5;color:#1a1a1a}
        .container{max-width:1200px;margin:0 auto;padding:24px}
        .header{display:flex;justify-content:space-between;align-items:center;margin-bottom:24px}
        .header h1{font-size:28px;font-weight:600}
        .btn{padding:10px 20px;border:none;border-radius:8px;cursor:pointer;font-size:14px;font-weight:500;text-decoration:none;display:inline-flex;align-items:center;gap:6px}
        .btn-primary{background:#0066cc;color:#fff}.btn-primary:hover{background:#0052a3}
        .btn-outline{background:transparent;border:1px solid #0066cc;color:#0066cc}.btn-outline:hover{background:#e8f4ff}
        .btn-danger{background:#dc3545;color:#fff}.btn-danger:hover{background:#b02a37}
        .btn-sm{padding:6px 12px;font-size:12px}
        #plugin-list{display:grid;gap:16px}
        .plugin-card{background:#fff;border-radius:12px;padding:20px;box-shadow:0 2px 8px rgba(0,0,0,0.08)}
        .plugin-card.active{border-left:4px solid #28a745}
        .plugin-card.inactive{border-left:4px solid #dc3545}
        .plugin-header{display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:8px}
        .plugin-info{display:flex;align-items:center;gap:10px}
        .plugin-name{font-size:16px;font-weight:600}
        .plugin-version{font-size:12px;color:#888;background:#f0f0f0;padding:2px 8px;border-radius:12px}
        .plugin-status{padding:4px 12px;border-radius:12px;font-size:12px;font-weight:500}
        .plugin-status.active{background:#d4edda;color:#155724}
        .plugin-status.inactive{background:#f8d7da;color:#721c24}
        .plugin-desc{font-size:14px;color:#555;margin-bottom:12px;line-height:1.4}
        .plugin-meta{display:flex;gap:16px;margin-bottom:12px;font-size:12px;color:#888}
        .meta-item{background:#f5f5f5;padding:2px 8px;border-radius:4px}
        .plugin-functions{background:#fafafa;border-radius:8px;padding:12px;margin-bottom:12px}
        .plugin-functions h4{font-size:13px;color:#333;margin-bottom:8px}
        .func-item{padding:8px;border-bottom:1px solid #eee;font-size:13px}
        .func-item:last-child{border-bottom:none}
        .func-name{font-family:monospace;color:#0066cc;font-weight:500}
        .func-desc{color:#666;margin-left:8px}
        .func-params{color:#999;font-size:12px;margin-left:8px}
        .plugin-actions{display:flex;gap:8px;padding-top:12px;border-top:1px solid #eee}
        .empty-state{text-align:center;padding:60px 24px;color:#666;background:#fff;border-radius:12px;box-shadow:0 2px 8px rgba(0,0,0,0.08)}
        .empty-state h3{margin-bottom:8px;color:#1a1a1a}
        .file-input{display:none}
        .nav-bar{display:flex;gap:12px;margin-bottom:24px}
        .nav-link{color:#0066cc;text-decoration:none;font-size:14px;padding:6px 12px;border-radius:6px;border:1px solid transparent}
        .nav-link:hover{background:#e8f4ff}
        .nav-link.active{border-color:#0066cc;font-weight:600}
        .alert{padding:12px 16px;border-radius:8px;margin-bottom:16px;font-size:14px}
        .alert-warning{background:#fff3cd;color:#856404;border:1px solid #ffeeba}
        .alert-success{background:#d4edda;color:#155724;border:1px solid #c3e6cb}
    </style>
</head>
<body>
    <div class="container">
        <div class="nav-bar">
            <a href="/suite/designer" class="nav-link">&larr; Designer</a>
            <a href="/suite/plugins" class="nav-link active">Plugins</a>
        </div>
        <div class="header">
            <h1>Plugin Manager</h1>
            <button class="btn btn-primary" onclick="document.getElementById('fileInput').click()">
                Upload Manifest
            </button>
            <input type="file" id="fileInput" class="file-input" accept=".json"
                   onchange="uploadManifest(this)">
        </div>
        <div id="plugin-list" hx-get="/api/ui/plugins" hx-trigger="load" hx-target="#plugin-list" hx-swap="outerHTML">
            <div style="text-align:center;padding:40px;color:#888">Loading plugins...</div>
        </div>
    </div>
    <script>
        function uploadManifest(input) {
            const file = input.files[0];
            if (!file) return;
            const formData = new FormData();
            formData.append('manifest', file);
            fetch('/api/ui/plugins/upload', {
                method: 'POST',
                body: formData
            }).then(r => r.text()).then(html => {
                document.getElementById('plugin-list').outerHTML = html;
                htmx.process(document.body);
            }).catch(e => alert('Upload failed: ' + e.message));
            input.value = '';
        }
    </script>
</body>
</html>"##,
    );
    Html(html)
}

pub fn configure_plugin_routes() -> Router<Arc<DesignerState>> {
    Router::new()
        .route("/api/ui/plugins", get(handle_list_plugins))
        .route("/api/ui/plugins/:id/toggle", post(handle_toggle_plugin))
        .route("/api/ui/plugins/:id", delete(handle_remove_plugin))
        .route("/api/ui/plugins/upload", post(handle_upload_manifest))
        .route("/suite/plugins", get(handle_plugins_page))
}
