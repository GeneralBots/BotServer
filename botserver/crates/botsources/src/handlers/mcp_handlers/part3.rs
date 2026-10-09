use super::*;

pub async fn handle_add_from_catalog(
    State(state): State<Arc<AppState>>,
    Form(form): Form<serde_json::Value>,
) -> impl IntoResponse {
    let server_id = form
        .get("server_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let bot_id = "default".to_string();
    let work_path = get_work_path_or_default(&state.get_work_path);
    let loader = make_mcp_loader(&state.mcp_loader, &work_path, &bot_id);

    if let Some(catalog) = load_mcp_servers_catalog() {
        if let Some(server) = catalog.mcp_servers.iter().find(|s| s.id == server_id) {
            let row = McpCsvRowData {
                name: server.name.clone(),
                connection_type: "sse".to_string(),
                command: server.provider.clone(),
                args: server.id.clone(),
                description: server.description.clone(),
                enabled: true,
                auth_type: None,
                auth_env: None,
                risk_level: None,
                requires_approval: false,
            };
            let _ = loader.add_server(&row);
        }
    }

    axum::response::Html(r##"<div class="mcp-add-success">Server added to configuration</div>"##.to_string())
}
