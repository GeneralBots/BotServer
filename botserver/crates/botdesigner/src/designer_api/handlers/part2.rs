use super::*;

pub async fn handle_list_dialogs(State(state): State<Arc<DesignerState>>) -> impl IntoResponse {
    let conn = state.conn.clone();

    let dialogs = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        diesel::sql_query(
            "SELECT id, name, content, updated_at FROM designer_dialogs ORDER BY updated_at DESC LIMIT 50",
        )
        .load::<DialogRow>(&mut db_conn)
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::new();
    html.push_str("<div class=\"dialogs-list\">");

    for dialog in &dialogs {
        html.push_str("<div class=\"dialog-card\" data-id=\"");
        html.push_str(&html_escape(&dialog.id));
        html.push_str("\">");
        html.push_str("<h4>");
        html.push_str(&html_escape(&dialog.name));
        html.push_str("</h4>");
        html.push_str("<span class=\"dialog-time\">");
        html.push_str(&format_relative_time(dialog.updated_at));
        html.push_str("</span>");
        html.push_str("</div>");
    }

    if dialogs.is_empty() {
        html.push_str("<div class=\"empty-state\">");
        html.push_str("<p>No dialogs yet</p>");
        html.push_str("</div>");
    }

    html.push_str("</div>");

    Html(html)
}
pub async fn handle_create_dialog(
    State(state): State<Arc<DesignerState>>,
    Json(payload): Json<SaveRequest>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let now = Utc::now();
    let dialog_id = Uuid::new_v4().to_string();
    let name = payload.name.unwrap_or_else(|| "New Dialog".to_string());
    let content = payload.content.unwrap_or_else(get_default_dialog_content);

    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Err(format!("Database error: {}", e));
            }
        };

        diesel::sql_query(
            "INSERT INTO designer_dialogs (id, name, description, bot_id, content, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
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
        .map_err(|e| format!("Create failed: {}", e))?;

        Ok(dialog_id)
    })
    .await
    .unwrap_or_else(|e| Err(format!("Task error: {}", e)));

    match result {
        Ok(id) => {
            let mut html = String::new();
            html.push_str("<div class=\"dialog-created\" data-id=\"");
            html.push_str(&html_escape(&id));
            html.push_str("\">");
            html.push_str("<span class=\"success\">Dialog created</span>");
            html.push_str("</div>");
            Html(html)
        }
        Err(e) => {
            let mut html = String::new();
            html.push_str("<div class=\"error\">");
            html.push_str(&html_escape(&e));
            html.push_str("</div>");
            Html(html)
        }
    }
}
pub async fn handle_get_dialog(
    State(state): State<Arc<DesignerState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.clone();

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
        .bind::<diesel::sql_types::Text, _>(&id)
        .get_result::<DialogRow>(&mut db_conn)
        .ok()
    })
    .await
    .unwrap_or(None);

    match dialog {
        Some(d) => {
            let mut html = String::new();
            html.push_str("<div class=\"dialog-content\" data-id=\"");
            html.push_str(&html_escape(&d.id));
            html.push_str("\">");
            html.push_str("<div class=\"dialog-header\">");
            html.push_str("<h3>");
            html.push_str(&html_escape(&d.name));
            html.push_str("</h3>");
            html.push_str("</div>");
            html.push_str("<div class=\"dialog-code\">");
            html.push_str("<pre>");
            html.push_str(&html_escape(&d.content));
            html.push_str("</pre>");
            html.push_str("</div>");
            html.push_str("</div>");
            Html(html)
        }
        None => Html("<div class=\"error\">Dialog not found</div>".to_string()),
    }
}
pub async fn handle_magic_suggestions(
    State(_state): State<Arc<DesignerState>>,
    Json(request): Json<MagicRequest>,
) -> impl IntoResponse {
    let mut suggestions = Vec::new();
    let nodes = &request.nodes;

    let has_hear = nodes.iter().any(|n| n.node_type == "HEAR");
    let has_talk = nodes.iter().any(|n| n.node_type == "TALK");
    let has_if = nodes
        .iter()
        .any(|n| n.node_type == "IF" || n.node_type == "SWITCH");
    let talk_count = nodes.iter().filter(|n| n.node_type == "TALK").count();

    if !has_hear && has_talk {
        suggestions.push(MagicSuggestion {
            suggestion_type: "ux".to_string(),
            title: "Add User Input".to_string(),
            description: "Your dialog has no HEAR nodes. Consider adding user input to make it interactive."
                .to_string(),
        });
    }

    if talk_count > 5 {
        suggestions.push(MagicSuggestion {
            suggestion_type: "ux".to_string(),
            title: "Break Up Long Responses".to_string(),
            description: "You have many TALK nodes. Consider grouping related messages or using a menu."
                .to_string(),
        });
    }

    if !has_if && nodes.len() > 3 {
        suggestions.push(MagicSuggestion {
            suggestion_type: "feature".to_string(),
            title: "Add Decision Logic".to_string(),
            description: "Add IF or SWITCH nodes to handle different user responses dynamically."
                .to_string(),
        });
    }

    if request.connections < (nodes.len() as i32 - 1) && nodes.len() > 1 {
        suggestions.push(MagicSuggestion {
            suggestion_type: "perf".to_string(),
            title: "Check Connections".to_string(),
            description: "Some nodes may not be connected. Ensure all nodes flow properly."
                .to_string(),
        });
    }

    if nodes.is_empty() {
        suggestions.push(MagicSuggestion {
            suggestion_type: "feature".to_string(),
            title: "Start with TALK".to_string(),
            description: "Begin your dialog with a TALK node to greet the user.".to_string(),
        });
    }

    suggestions.push(MagicSuggestion {
        suggestion_type: "a11y".to_string(),
        title: "Use Clear Language".to_string(),
        description: "Keep messages short and clear. Avoid jargon for better accessibility."
            .to_string(),
    });

    Json(suggestions)
}
