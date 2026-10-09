use super::*;

pub async fn handle_list_mcp_servers_json(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BotQuery>,
) -> impl IntoResponse {
    let bot_id = params.bot_id.unwrap_or_else(|| "default".to_string());
    let work_path = get_work_path_or_default(&state.get_work_path);

    let loader = make_mcp_loader(&state.mcp_loader, &work_path, &bot_id);
    let scan_result = loader.load();

    let servers: Vec<McpServerResponse> = scan_result
        .servers
        .iter()
        .map(|s| McpServerResponse {
            id: s.id.clone(),
            name: s.name.clone(),
            description: s.description.clone(),
            server_type: s.server_type.clone(),
            status: s.status.clone(),
            enabled: s.enabled,
            tools_count: s.tools.len(),
            source: "directory".to_string(),
            tags: Vec::new(),
            requires_approval: s.tools.iter().any(|t| t.requires_approval),
        })
        .collect();

    Json(ApiResponse::success(servers))
}
pub async fn handle_add_mcp_server(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BotQuery>,
    Json(request): Json<crate::types::AddMcpServerRequest>,
) -> impl IntoResponse {
    let bot_id = params.bot_id.unwrap_or_else(|| "default".to_string());
    let work_path = get_work_path_or_default(&state.get_work_path);

    let loader = make_mcp_loader(&state.mcp_loader, &work_path, &bot_id);

    let (conn_type, command, args) = match &request.connection {
        crate::types::McpConnectionRequest::Stdio { command, args } => {
            ("stdio".to_string(), command.clone(), args.join(" "))
        }
        crate::types::McpConnectionRequest::Http { url, .. } => ("http".to_string(), url.clone(), String::new()),
        crate::types::McpConnectionRequest::WebSocket { url } => {
            ("websocket".to_string(), url.clone(), String::new())
        }
    };

    let (auth_type, auth_env) = match &request.auth {
        Some(crate::types::McpAuthRequest::ApiKey { key_env, .. }) => {
            (Some("api_key".to_string()), Some(key_env.clone()))
        }
        Some(crate::types::McpAuthRequest::Bearer { token_env }) => {
            (Some("bearer".to_string()), Some(token_env.clone()))
        }
        _ => (None, None),
    };

    let row = McpCsvRowData {
        name: request.name.clone(),
        connection_type: conn_type,
        command,
        args,
        description: request.description.clone().unwrap_or_default(),
        enabled: request.enabled.unwrap_or(true),
        auth_type,
        auth_env,
        risk_level: Some("medium".to_string()),
        requires_approval: request.requires_approval.unwrap_or(false),
    };

    match loader.add_server(&row) {
        Ok(()) => {
            log::info!("Added MCP server '{}' to mcp.csv", request.name);
            Json(ApiResponse::success(format!(
                "MCP server '{}' created successfully",
                request.name
            )))
            .into_response()
        }
        Err(e) => {
            log::error!("Failed to create MCP server: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(&format!(
                    "Failed to create MCP server: {}", e
                ))),
            )
                .into_response()
        }
    }
}
pub async fn handle_get_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<BotQuery>,
) -> impl IntoResponse {
    let bot_id = params.bot_id.unwrap_or_else(|| "default".to_string());
    let work_path = get_work_path_or_default(&state.get_work_path);

    let loader = make_mcp_loader(&state.mcp_loader, &work_path, &bot_id);

    match loader.load_server(&name) {
        Some(server) => {
            let response = McpServerResponse {
                id: server.id,
                name: server.name,
                description: server.description,
                server_type: server.server_type,
                status: server.status,
                enabled: server.enabled,
                tools_count: server.tools.len(),
                source: "directory".to_string(),
                tags: Vec::new(),
                requires_approval: server.tools.iter().any(|t| t.requires_approval),
            };
            Json(ApiResponse::success(response)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<McpServerResponse>::error(&format!(
                "MCP server '{}' not found", name
            ))),
        )
            .into_response(),
    }
}
pub async fn handle_update_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<BotQuery>,
    Json(request): Json<crate::types::AddMcpServerRequest>,
) -> impl IntoResponse {
    let bot_id = params.bot_id.unwrap_or_else(|| "default".to_string());
    let work_path = get_work_path_or_default(&state.get_work_path);

    let loader = make_mcp_loader(&state.mcp_loader, &work_path, &bot_id);

    let _ = loader.remove_server(&name);

    let (conn_type, command, args) = match &request.connection {
        crate::types::McpConnectionRequest::Stdio { command, args } => {
            ("stdio".to_string(), command.clone(), args.join(" "))
        }
        crate::types::McpConnectionRequest::Http { url, .. } => ("http".to_string(), url.clone(), String::new()),
        crate::types::McpConnectionRequest::WebSocket { url } => {
            ("websocket".to_string(), url.clone(), String::new())
        }
    };

    let (auth_type, auth_env) = match &request.auth {
        Some(crate::types::McpAuthRequest::ApiKey { key_env, .. }) => {
            (Some("api_key".to_string()), Some(key_env.clone()))
        }
        Some(crate::types::McpAuthRequest::Bearer { token_env }) => {
            (Some("bearer".to_string()), Some(token_env.clone()))
        }
        _ => (None, None),
    };

    let row = McpCsvRowData {
        name: request.name.clone(),
        connection_type: conn_type,
        command,
        args,
        description: request.description.clone().unwrap_or_default(),
        enabled: request.enabled.unwrap_or(true),
        auth_type,
        auth_env,
        risk_level: Some("medium".to_string()),
        requires_approval: request.requires_approval.unwrap_or(false),
    };

    match loader.add_server(&row) {
        Ok(()) => Json(ApiResponse::success(format!(
            "MCP server '{}' updated successfully", request.name
        ))),
        Err(e) => Json(ApiResponse::<String>::error(&format!(
            "Failed to update MCP server: {}", e
        ))),
    }
}
pub async fn handle_delete_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<BotQuery>,
) -> impl IntoResponse {
    let bot_id = params.bot_id.unwrap_or_else(|| "default".to_string());
    let work_path = get_work_path_or_default(&state.get_work_path);

    let loader = make_mcp_loader(&state.mcp_loader, &work_path, &bot_id);

    match loader.remove_server(&name) {
        Ok(true) => Json(ApiResponse::success(format!(
            "MCP server '{}' deleted successfully", name
        )))
        .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<String>::error(&format!(
                "MCP server '{}' not found", name
            ))),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(&format!(
                "Failed to delete MCP server: {}", e
            ))),
        )
            .into_response(),
    }
}
pub(super) fn set_mcp_server_enabled(work_path: &str, bot_id: &str, name: &str, enabled: bool) -> Result<(), String> {
    let csv_path = std::path::PathBuf::from(work_path)
        .join(format!("{}.gbai", bot_id))
        .join("mcp.csv");

    if !csv_path.exists() {
        return Err("mcp.csv not found".to_string());
    }

    let content = std::fs::read_to_string(&csv_path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = Vec::new();
    let mut found = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('/') {
            lines.push(line.to_string());
            continue;
        }

        let columns = parse_csv_columns(&trimmed);
        let server_name = columns.first().map(|s| s.trim().trim_matches('"')).unwrap_or("");

        if server_name == name {
            found = true;
            let new_columns: Vec<String> = columns.iter().enumerate().map(|(idx, col)| {
                if idx == 5 {
                    enabled.to_string()
                } else {
                    col.to_string()
                }
            }).collect();
            lines.push(new_columns.join(","));
        } else {
            lines.push(line.to_string());
        }
    }

    if !found {
        return Err(format!("MCP server '{}' not found", name));
    }

    std::fs::write(&csv_path, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(())
}
pub(super) fn parse_csv_columns(line: &str) -> Vec<String> {
    let mut columns = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_quotes => { in_quotes = true; }
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            }
            ',' if !in_quotes => {
                columns.push(current.clone());
                current.clear();
            }
            _ => { current.push(c); }
        }
    }
    columns.push(current);
    columns
}
pub async fn handle_enable_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<BotQuery>,
) -> impl IntoResponse {
    let bot_id = params.bot_id.unwrap_or_else(|| "default".to_string());
    let work_path = get_work_path_or_default(&state.get_work_path);

    match set_mcp_server_enabled(&work_path, &bot_id, &name, true) {
        Ok(()) => Json(ApiResponse::success(format!("MCP server '{}' enabled", name))),
        Err(e) => Json(ApiResponse::<String>::error(&e)),
    }
}
