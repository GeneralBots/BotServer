use super::*;

pub fn configure_designer_routes() -> Router<Arc<DesignerState>> {
    Router::new()
        .route(ROUTE_FILES, get(handle_list_files))
        .route(ROUTE_LOAD, get(handle_load_file))
        .route(ROUTE_SAVE, post(handle_save))
        .route(ROUTE_VALIDATE, post(handle_validate))
        .route(ROUTE_EXPORT, get(handle_export))
        .route(ROUTE_DIALOGS, get(handle_list_dialogs).post(handle_create_dialog))
        .route(ROUTE_DIALOG_BY_ID, get(handle_get_dialog))
        .route(ROUTE_MODIFY, post(crate::designer_api::llm_integration::handle_designer_modify))
        .route(ROUTE_MAGIC, post(handle_magic_suggestions))
        .route(ROUTE_EDITOR_MAGIC, post(crate::designer_api::llm_integration::handle_editor_magic))
}
pub async fn handle_list_files(State(state): State<Arc<DesignerState>>) -> impl IntoResponse {
    let conn = state.conn.clone();

    let files = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return get_default_files();
            }
        };

        let result: Result<Vec<DialogRow>, _> = diesel::sql_query(
            "SELECT id, name, content, updated_at FROM designer_dialogs ORDER BY updated_at DESC LIMIT 50",
        )
        .load(&mut db_conn);

        match result {
            Ok(dialogs) if !dialogs.is_empty() => dialogs
                .into_iter()
                .map(|d| (d.id, d.name, d.updated_at))
                .collect(),
            _ => get_default_files(),
        }
    })
    .await
    .unwrap_or_else(|_| get_default_files());

    let mut html = String::new();
    html.push_str("<div class=\"file-list\">");

    for (id, name, updated_at) in &files {
        let time_str = format_relative_time(*updated_at);
        html.push_str("<div class=\"file-item\" data-id=\"");
        html.push_str(&html_escape(id));
        html.push_str("\" onclick=\"selectFile(this)\">");
        html.push_str("<div class=\"file-icon\">");
        html.push_str("<svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\">");
        html.push_str(
            "<path d=\"M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z\"></path>",
        );
        html.push_str("<polyline points=\"14 2 14 8 20 8\"></polyline>");
        html.push_str("</svg>");
        html.push_str("</div>");
        html.push_str("<div class=\"file-info\">");
        html.push_str("<span class=\"file-name\">");
        html.push_str(&html_escape(name));
        html.push_str("</span>");
        html.push_str("<span class=\"file-time\">");
        html.push_str(&html_escape(&time_str));
        html.push_str("</span>");
        html.push_str("</div>");
        html.push_str("</div>");
    }

    if files.is_empty() {
        html.push_str("<div class=\"empty-state\">");
        html.push_str("<p>No dialog files found</p>");
        html.push_str("<p class=\"hint\">Create a new dialog to get started</p>");
        html.push_str("</div>");
    }

    html.push_str("</div>");

    Html(html)
}
pub async fn handle_load_file(
    State(state): State<Arc<DesignerState>>,
    Query(params): Query<FileQuery>,
) -> impl IntoResponse {
    let file_path = params.path.unwrap_or_else(|| "welcome".to_string());

    let content = if let Some(bucket) = params.bucket {
        match (state.load_from_drive)(&bucket, &file_path) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to load file from drive: {}", e);
                get_default_dialog_content()
            }
        }
    } else {
        let conn = state.conn.clone();
        let file_id = file_path;

        let dialog = tokio::task::spawn_blocking(move || {
            let mut db_conn = match conn.get() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("DB connection error: {}", e);
                    return None;
                }
            };

            diesel::sql_query(
                "SELECT id, name, content, updated_at FROM designer_dialogs WHERE id = $1",
            )
            .bind::<diesel::sql_types::Text, _>(&file_id)
            .get_result::<DialogRow>(&mut db_conn)
            .ok()
        })
        .await
        .unwrap_or(None);

        match dialog {
            Some(d) => d.content,
            None => get_default_dialog_content(),
        }
    };

    let mut html = String::new();
    html.push_str("<div class=\"canvas-loaded\" data-content=\"");
    html.push_str(&html_escape(&content));
    html.push_str("\">");

    let nodes = parse_basic_to_nodes(&content);
    for node in &nodes {
        html.push_str(&format_node_html(node));
    }

    html.push_str("</div>");
    html.push_str("<script>initializeCanvas();</script>");

    Html(html)
}
pub async fn handle_save(
    State(state): State<Arc<DesignerState>>,
    Json(payload): Json<SaveRequest>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let now = Utc::now();
    let name = payload.name.unwrap_or_else(|| "Untitled".to_string());
    let content = payload.content.unwrap_or_default();
    let dialog_id = Uuid::new_v4().to_string();

    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Err(format!("Database error: {}", e));
            }
        };

        diesel::sql_query(
            "INSERT INTO designer_dialogs (id, name, description, bot_id, content, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (id) DO UPDATE SET content = $5, updated_at = $8",
        )
        .bind::<diesel::sql_types::Text, _>(&dialog_id)
        .bind::<diesel::sql_types::Text, _>(&name)
        .bind::<diesel::sql_types::Text, _>("")
        .bind::<diesel::sql_types::Text, _>("default")
        .bind::<diesel::sql_types::Text, _>(&content)
        .bind::<diesel::sql_types::Bool, _>(false)
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .execute(&mut db_conn)
        .map_err(|e| format!("Save failed: {}", e))?;

        Ok(())
    })
    .await
    .unwrap_or_else(|e| Err(format!("Task error: {}", e)));

    match result {
        Ok(()) => {
            let mut html = String::new();
            html.push_str("<div class=\"save-result success\">");
            html.push_str("<span class=\"save-icon\">*</span>");
            html.push_str("<span class=\"save-message\">Saved successfully</span>");
            html.push_str("</div>");
            Html(html)
        }
        Err(e) => {
            let mut html = String::new();
            html.push_str("<div class=\"save-result error\">");
            html.push_str("<span class=\"save-icon\">x</span>");
            html.push_str("<span class=\"save-message\">Save failed: ");
            html.push_str(&html_escape(&e));
            html.push_str("</span>");
            html.push_str("</div>");
            Html(html)
        }
    }
}
pub async fn handle_validate(
    State(_state): State<Arc<DesignerState>>,
    Json(payload): Json<ValidateRequest>,
) -> impl IntoResponse {
    let content = payload.content.unwrap_or_default();
    let validation = validate_basic_code(&content);

    let mut html = String::new();
    html.push_str("<div class=\"validation-result\">");

    if validation.valid {
        html.push_str("<div class=\"validation-success\">");
        html.push_str("<span class=\"validation-icon\">*</span>");
        html.push_str("<span class=\"validation-text\">Dialog is valid</span>");
        html.push_str("</div>");
    } else {
        if !validation.errors.is_empty() {
            html.push_str("<div class=\"validation-errors\">");
            html.push_str("<div class=\"validation-header\">");
            html.push_str("<span class=\"validation-icon\">✗</span>");
            html.push_str("<span class=\"validation-text\">");
            html.push_str(&validation.errors.len().to_string());
            html.push_str(" error(s) found</span>");
            html.push_str("</div>");
            html.push_str("<ul class=\"error-list\">");
            for error in &validation.errors {
                html.push_str("<li class=\"error-item\" data-line=\"");
                html.push_str(&error.line.to_string());
                html.push_str("\">");
                html.push_str("<span class=\"error-line\">Line ");
                html.push_str(&error.line.to_string());
                html.push_str(":</span> ");
                html.push_str(&html_escape(&error.message));
                html.push_str("</li>");
            }
        } else if !validation.warnings.is_empty() {
            html.push_str("<div class=\"validation-warnings\">");
            html.push_str("<div class=\"validation-header\">");
            html.push_str("<span class=\"validation-icon\">!</span>");
            html.push_str("<span class=\"validation-text\">");
            html.push_str(&validation.warnings.len().to_string());
            html.push_str(" warning(s)</span>");
            html.push_str("</div>");
            html.push_str("<ul class=\"warning-list\">");
            for warning in &validation.warnings {
                html.push_str("<li class=\"warning-item\">");
                html.push_str("<span class=\"warning-line\">Line ");
                html.push_str(&warning.line.to_string());
                html.push_str(":</span> ");
                html.push_str(&html_escape(&warning.message));
                html.push_str("</li>");
            }
        }

        if !validation.errors.is_empty() || !validation.warnings.is_empty() {
            html.push_str("</ul>");
            html.push_str("</div>");
        }
    }

    html.push_str("</div>");

    Html(html)
}
pub async fn handle_export(
    State(_state): State<Arc<DesignerState>>,
    Query(params): Query<FileQuery>,
) -> impl IntoResponse {
    let _file_id = params.path.unwrap_or_else(|| "dialog".to_string());

    Html("<script>alert('Export started. File will download shortly.');</script>".to_string())
}
