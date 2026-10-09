use super::*;

pub async fn list_templates_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return axum::response::Html(r#"<div class="template-item">Please sign in</div>"#.to_string()),
    };

    let pool = state.pool.clone();
    let temps_result = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {}", e))?;
        diesel::sql_query("SELECT id, name, subject_template, body_html_template FROM email_templates WHERE user_id = $1 OR user_id IS NULL ORDER BY name ASC")
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .get_results::<TemplateRow>(&mut db_conn)
            .map_err(|e| format!("Failed to load templates: {}", e))
    }).await;

    let temps = match temps_result {
        Ok(Ok(list)) => list,
        _ => Vec::new(),
    };

    if temps.is_empty() {
        return axum::response::Html(r#"<div class="template-item" onclick="useTemplate('welcome')"><h4>Welcome Email</h4><p>Standard welcome message for new contacts</p></div><div class="template-item" onclick="useTemplate('followup')"><h4>Follow Up</h4><p>General follow-up template</p></div><div class="template-item" onclick="useTemplate('meeting')"><h4>Meeting Request</h4><p>Request a meeting with scheduling options</p></div><p class="text-sm text-gray" style="margin-top: 1rem; text-align: center;">Click a template to use it</p>"#.to_string());
    }

    let mut html = String::new();
    use std::fmt::Write;
    for temp in temps {
        let _ = write!(html, r##"<div class="template-item" onclick="useTemplate('{}')"><h4>{}</h4><p class="text-sm text-gray">{}</p></div>"##, temp.id, temp.name, temp.subject_template);
    }
    let _ = writeln!(html, r#"<p class="text-sm text-gray" style="margin-top: 1rem; text-align: center;">Click a template to use it</p>"#);

    axum::response::Html(html)
}
pub async fn list_signatures_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return axum::response::Html(r#"<div class="signature-item">Please sign in</div>"#.to_string()),
    };

    let pool = state.pool.clone();
    let sigs_result = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {}", e))?;
        diesel::sql_query("SELECT id, name, content_html FROM email_signatures WHERE user_id = $1 AND is_active = true ORDER BY name ASC")
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .get_results::<SignatureRow>(&mut db_conn)
            .map_err(|e| format!("Failed to load signatures: {}", e))
    }).await;

    let sigs = match sigs_result {
        Ok(Ok(list)) => list,
        _ => Vec::new(),
    };

    if sigs.is_empty() {
        return axum::response::Html(r#"<div class="signature-item" onclick="useSignature('default')"><h4>Default Signature</h4><p class="text-sm text-gray">Best regards,<br>Your Name</p></div><div class="signature-item" onclick="useSignature('formal')"><h4>Formal Signature</h4><p class="text-sm text-gray">Sincerely,<br>Your Name<br>Title | Company</p></div><p class="text-sm text-gray" style="margin-top: 1rem; text-align: center;">Click a signature to insert it</p>"#.to_string());
    }

    let mut html = String::new();
    use std::fmt::Write;
    for sig in sigs {
        let _ = write!(html, r##"<div class="signature-item" onclick="useSignature('{}')"><h4>{}</h4><div class="text-sm text-gray">{}</div></div>"##, sig.id, sig.name, sig.content_html);
    }
    let _ = writeln!(html, r#"<p class="text-sm text-gray" style="margin-top: 1rem; text-align: center;">Click a signature to insert it</p>"#);

    axum::response::Html(html)
}
pub async fn list_rules_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return axum::response::Html(r#"<div class="rule-item">Please sign in</div>"#.to_string()),
    };

    let pool = state.pool.clone();
    let rules_result = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {}", e))?;
        diesel::sql_query("SELECT id, name, conditions_json, actions_json, is_active FROM email_rules WHERE user_id = $1 ORDER BY name ASC")
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .get_results::<RuleRow>(&mut db_conn)
            .map_err(|e| format!("Failed to load rules: {}", e))
    }).await;

    let rules = match rules_result {
        Ok(Ok(list)) => list,
        _ => Vec::new(),
    };

    if rules.is_empty() {
        return axum::response::Html(r#"<div class="rule-item"><div class="rule-header"><span class="rule-name">Auto-archive newsletters</span><label class="toggle-label"><input type="checkbox" checked><span class="toggle-switch"></span></label></div><p class="text-sm text-gray">From: *@newsletter.* → Archive</p></div><div class="rule-item"><div class="rule-header"><span class="rule-name">Label work emails</span><label class="toggle-label"><input type="checkbox" checked><span class="toggle-switch"></span></label></div><p class="text-sm text-gray">From: *@company.com → Label: Work</p></div>"#.to_string());
    }

    let mut html = String::new();
    use std::fmt::Write;
    for rule in rules {
        let checked = if rule.is_active { "checked" } else { "" };
        let _ = write!(html, r##"<div class="rule-item"><div class="rule-header"><span class="rule-name">{}</span><label class="toggle-label"><input type="checkbox" {}><span class="toggle-switch"></span></label></div><p class="text-sm text-gray">If: {} → Action: {}</p></div>"##, rule.name, checked, rule.conditions_json, rule.actions_json);
    }

    axum::response::Html(html)
}
pub async fn create_rule(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, axum::Json(serde_json::json!({ "success": false, "error": "Unauthorized" }))).into_response(),
    };

    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed Rule").to_string();
    let condition = payload.get("condition").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let action = payload.get("action").and_then(|v| v.as_str()).unwrap_or("archive").to_string();

    let pool = state.pool.clone();
    let insert_result = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {}", e))?;
        let bot_id = Uuid::nil();
        diesel::sql_query("INSERT INTO email_rules (id, user_id, bot_id, name, conditions_json, actions_json, is_active, priority, stop_processing, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())")
            .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .bind::<diesel::sql_types::Varchar, _>(&name)
            .bind::<diesel::sql_types::Text, _>(&condition)
            .bind::<diesel::sql_types::Text, _>(&action)
            .bind::<diesel::sql_types::Bool, _>(true)
            .bind::<diesel::sql_types::Integer, _>(0)
            .bind::<diesel::sql_types::Bool, _>(true)
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to insert rule: {}", e))
    }).await;

    match insert_result {
        Ok(Ok(_)) => (StatusCode::OK, axum::Json(serde_json::json!({ "success": true }))).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(serde_json::json!({ "success": false, "error": "Database error" }))).into_response(),
    }
}
pub async fn create_template(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, axum::Json(serde_json::json!({ "success": false, "error": "Unauthorized" }))).into_response(),
    };

    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed Template").to_string();
    let subject = payload.get("subject").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let body = payload.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let pool = state.pool.clone();
    let insert_result = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {}", e))?;
        let bot_id = Uuid::nil();
        diesel::sql_query("INSERT INTO email_templates (id, bot_id, user_id, name, subject_template, body_html_template, variables_json, is_shared, usage_count, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())")
            .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Varchar, _>(&name)
            .bind::<diesel::sql_types::Text, _>(&subject)
            .bind::<diesel::sql_types::Text, _>(&body)
            .bind::<diesel::sql_types::Text, _>("{}")
            .bind::<diesel::sql_types::Bool, _>(false)
            .bind::<diesel::sql_types::Integer, _>(0)
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to insert template: {}", e))
    }).await;

    match insert_result {
        Ok(Ok(_)) => (StatusCode::OK, axum::Json(serde_json::json!({ "success": true }))).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(serde_json::json!({ "success": false, "error": "Database error" }))).into_response(),
    }
}
