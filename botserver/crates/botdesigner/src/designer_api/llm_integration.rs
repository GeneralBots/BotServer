use super::types::*;
use crate::DesignerState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use diesel::RunQueryDsl;
use std::fmt::Write;
use std::sync::Arc;
use uuid::Uuid;

pub async fn handle_editor_magic(
    State(state): State<Arc<DesignerState>>,
    Json(request): Json<EditorMagicRequest>,
) -> impl IntoResponse {
    let code = request.code;

    if code.trim().is_empty() {
        return Json(EditorMagicResponse {
            improved_code: None,
            explanation: Some("No code provided".to_string()),
            suggestions: None,
        });
    }

    let prompt = format!(
        r#"You are reviewing this HTMX application code. Analyze and improve it.

Focus on:
- Better HTMX patterns (reduce JS, use hx-* attributes properly)
- Accessibility (ARIA labels, keyboard navigation, semantic HTML)
- Performance (lazy loading, efficient selectors)
- UX (loading states, error handling, user feedback)
- Code organization (clean structure, no comments needed)

Current code:
```
{code}
```

Respond with JSON only:
{{
  "improved_code": "the improved code here",
  "explanation": "brief explanation of changes made"
}}

If the code is already good, respond with:
{{
  "improved_code": null,
  "explanation": "Code looks good, no improvements needed"
}}"#
    );

    let config = serde_json::json!({
        "temperature": 0.3,
        "max_tokens": 4000
    });

    match (state.call_llm)(&prompt, &config) {
        Ok(response) => {
            if let Ok(result) = serde_json::from_str::<EditorMagicResponse>(&response) {
                return Json(result);
            }
            return Json(EditorMagicResponse {
                improved_code: Some(response),
                explanation: Some("AI suggestions".to_string()),
                suggestions: None,
            });
        }
        Err(e) => {
            log::warn!("LLM call failed: {e}");
        }
    }

    let mut suggestions = Vec::new();

    if !code.contains("hx-") {
        suggestions.push(MagicSuggestion {
            suggestion_type: "ux".to_string(),
            title: "Use HTMX attributes".to_string(),
            description: "Consider using hx-get, hx-post instead of JavaScript fetch calls."
                .to_string(),
        });
    }

    if !code.contains("hx-indicator") {
        suggestions.push(MagicSuggestion {
            suggestion_type: "ux".to_string(),
            title: "Add loading indicators".to_string(),
            description: "Use hx-indicator to show loading state during requests.".to_string(),
        });
    }

    if !code.contains("aria-") && !code.contains("role=") {
        suggestions.push(MagicSuggestion {
            suggestion_type: "a11y".to_string(),
            title: "Improve accessibility".to_string(),
            description: "Add ARIA labels and roles for screen reader support.".to_string(),
        });
    }

    if code.contains("onclick=") || code.contains("addEventListener") {
        suggestions.push(MagicSuggestion {
            suggestion_type: "perf".to_string(),
            title: "Replace JS with HTMX".to_string(),
            description: "HTMX can handle most interactions without custom JavaScript.".to_string(),
        });
    }

    Json(EditorMagicResponse {
        improved_code: None,
        explanation: None,
        suggestions: if suggestions.is_empty() {
            None
        } else {
            Some(suggestions)
        },
    })
}

pub async fn handle_designer_modify(
    State(state): State<Arc<DesignerState>>,
    Json(request): Json<DesignerModifyRequest>,
) -> impl IntoResponse {
    let app = &request.app_name;
    let msg_preview = botlib::text::truncate_at_char_boundary(&request.message, 100);
    log::info!("Designer modify request for app '{app}': {msg_preview}");

    let session = match get_designer_session(&state) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(DesignerModifyResponse {
                    success: false,
                    message: "Authentication required".to_string(),
                    changes: Vec::new(),
                    suggestions: Vec::new(),
                    error: Some(e.to_string()),
                }),
            );
        }
    };

    match process_designer_modification(&state, &request, &session).await {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(e) => {
            log::error!("Designer modification failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DesignerModifyResponse {
                    success: false,
                    message: "Failed to process modification".to_string(),
                    changes: Vec::new(),
                    suggestions: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

struct DesignerSession;

fn get_designer_session(state: &DesignerState) -> Result<DesignerSession, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = state.conn.get()?;

    let count: i64 = diesel::sql_query("SELECT COUNT(*) as count FROM bots")
        .get_result::<CountRow>(&mut conn)
        .map_err(|_| "No bot found for designer session")?
        .count;

    if count == 0 {
        return Err("No bot found for designer session".into());
    }

    Ok(DesignerSession)
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

async fn process_designer_modification(
    state: &DesignerState,
    request: &DesignerModifyRequest,
    session: &DesignerSession,
) -> Result<DesignerModifyResponse, Box<dyn std::error::Error + Send + Sync>> {
    let prompt = build_designer_prompt(state, request);
    let llm_response = call_designer_llm(state, &prompt)?;
    let (changes, message, suggestions) =
        parse_and_apply_changes(state, request, &llm_response, session).await?;

    Ok(DesignerModifyResponse {
        success: true,
        message,
        changes,
        suggestions,
        error: None,
    })
}

fn build_designer_prompt(state: &DesignerState, request: &DesignerModifyRequest) -> String {
    let context_info = request
        .context
        .as_ref()
        .map(|ctx| {
            let mut info = String::new();
            if let Some(ref html) = ctx.page_html {
                let _ = writeln!(
                    info,
                    "\nCurrent page HTML (first 500 chars):\n{}",
                    botlib::text::truncate_at_char_boundary(html, 500)
                );
            }
            if let Some(ref tables) = ctx.tables {
                let _ = writeln!(info, "\nAvailable tables: {}", tables.join(", "));
            }
            info
        })
        .unwrap_or_default();

    let error_context = (state.get_designer_error_context)(&request.app_name).unwrap_or_default();

    format!(
        r#"You are a Designer AI assistant helping modify an HTMX-based application.

App Name: {}
Current Page: {}
{}
{}
User Request: "{}"

Analyze the request and respond with JSON describing the changes needed:
{{
  "understanding": "brief description of what user wants",
  "changes": [
    {{
      "type": "modify_html|add_field|remove_field|add_table|modify_style|add_page",
      "file": "filename.html or styles.css",
      "description": "what this change does",
      "code": "the new/modified code snippet"
    }}
  ],
  "message": "friendly response to user explaining what was done",
  "suggestions": ["optional follow-up suggestions"]
}}

Guidelines:
- Use HTMX attributes (hx-get, hx-post, hx-target, hx-swap, hx-trigger)
- Keep styling minimal and consistent
- API endpoints follow pattern: /api/db/{{table_name}}
- Forms should use hx-post for submissions
- Lists should use hx-get with pagination
- IMPORTANT: Use RELATIVE paths for app assets (styles.css, app.js, NOT /static/styles.css)
- For HTMX, use LOCAL: <script src="/js/vendor/htmx.min.js"></script> (NO external CDN)
- CSS link should be: <link rel="stylesheet" href="styles.css">

Respond with valid JSON only."#,
        request.app_name,
        request.current_page.as_deref().unwrap_or("index.html"),
        context_info,
        error_context,
        request.message
    )
}

fn call_designer_llm(
    state: &DesignerState,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let model = (state.get_config)(&Uuid::nil().to_string(), "llm-model", Some("claude-sonnet-4-20250514"))
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

    let api_key = (state.get_config)(&Uuid::nil().to_string(), "llm-key", None)
        .unwrap_or_default();

    let messages = serde_json::json!({
        "model": model,
        "api_key": api_key,
        "messages": [
            {"role": "system", "content": "You are a web designer AI. Respond only with valid JSON."},
            {"role": "user", "content": prompt}
        ]
    });

    let response_text = match (state.call_llm)(prompt, &messages) {
        Ok(text) => text,
        Err(_) => String::from("{}"),
    };

    let json_text = if response_text.contains("```json") {
        response_text
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(&response_text)
            .trim()
            .to_string()
    } else if response_text.contains("```") {
        response_text
            .split("```")
            .nth(1)
            .unwrap_or(&response_text)
            .trim()
            .to_string()
    } else {
        response_text
    };

    Ok(json_text)
}

async fn parse_and_apply_changes(
    state: &DesignerState,
    request: &DesignerModifyRequest,
    llm_response: &str,
    _session: &DesignerSession,
) -> Result<(Vec<DesignerChange>, String, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    #[derive(serde::Deserialize)]
    struct LlmChangeResponse {
        _understanding: Option<String>,
        changes: Option<Vec<LlmChange>>,
        message: Option<String>,
        suggestions: Option<Vec<String>>,
    }

    #[derive(serde::Deserialize)]
    struct LlmChange {
        #[serde(rename = "type")]
        change_type: String,
        file: String,
        description: String,
        code: Option<String>,
    }

    let parsed: LlmChangeResponse = serde_json::from_str(llm_response).unwrap_or_else(|_| LlmChangeResponse {
        _understanding: Some("Could not parse LLM response".to_string()),
        changes: None,
        message: Some("I understood your request but encountered an issue processing it. Could you try rephrasing?".to_string()),
        suggestions: Some(vec!["Try being more specific".to_string()]),
    });

    let mut applied_changes = Vec::new();

    if let Some(changes) = parsed.changes {
        for change in changes {
            if let Some(ref code) = change.code {
                match apply_file_change(state, &request.app_name, &change.file, code).await {
                    Ok(()) => {
                        applied_changes.push(DesignerChange {
                            change_type: change.change_type,
                            file_path: change.file,
                            description: change.description,
                            preview: Some(code[..code.len().min(200)].to_string()),
                        });
                    }
                    Err(e) => {
                        let file = &change.file;
                        log::warn!("Failed to apply change to {file}: {e}");
                    }
                }
            }
        }
    }

    let message = parsed.message.unwrap_or_else(|| {
        if applied_changes.is_empty() {
            "I couldn't make any changes. Could you provide more details?".to_string()
        } else {
            format!(
                "Done! I made {} change(s) to your app.",
                applied_changes.len()
            )
        }
    });

    let suggestions = parsed.suggestions.unwrap_or_default();

    Ok((applied_changes, message, suggestions))
}

pub async fn apply_file_change(
    state: &DesignerState,
    app_name: &str,
    file_name: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bucket_name = state.bucket_name.clone();
    let sanitized_name = bucket_name.trim_end_matches(".gbai").to_string();

    let site_path = state
        .site_path
        .clone()
        .unwrap_or_else(|| format!("{}/sites", (state.get_stack_path)()));

    let local_path = format!("{site_path}/{org_id}.gborg/{sanitized_name}.gbai/{sanitized_name}.gbapp/{app_name}/{file_name}", org_id = botcore::shared::utils::current_org_id());
    if let Some(parent) = std::path::Path::new(&local_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&local_path, content)?;
    log::info!("Designer updated local file: {local_path}");

    let file_path = format!("{}.gbapp/{}/{}", sanitized_name, app_name, file_name);
    let content_type = (state.get_content_type)(file_name);

    match (state.write_to_drive)(&bucket_name, &file_path, content.as_bytes(), content_type) {
        Ok(()) => {
            log::info!("Designer synced to drive: {}/{}/{}", bucket_name, sanitized_name, file_path);
        }
        Err(e) => {
            log::warn!("Designer drive sync failed (local write succeeded): {e}");
        }
    }

    Ok(())
}
