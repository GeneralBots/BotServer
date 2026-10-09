use super::*;

pub async fn ai_chat_handler(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<AIChatBody>,
) -> Result<Json<AIChatResponse>, (StatusCode, Json<serde_json::Value>)> {
    let provider = state
        .llm_provider
        .as_ref()
        .ok_or_else(|| err(StatusCode::SERVICE_UNAVAILABLE, "LLM provider not configured"))?;

    let drive = get_drive(&state)?;
    let scope = req.scope.unwrap_or_default();
    let uid = effective_uid(&state, &user, req.user_id.as_deref());
    let bucket = resolve_bucket(&state, req.bucket.as_deref(), &scope, Some(uid.as_str()), None, &user)?;

    let mut files_context = String::new();
    if let Ok(objects) = drive.list_objects_with_metadata(&bucket, None).await {
        files_context.push_str("Files currently in this storage:\n");
        for obj in objects.iter().take(20) {
            files_context.push_str(&format!("- Name: {}, Size: {} bytes\n", obj.key, obj.size));
        }
    }

    let mut file_content_context = String::new();
    if let Some(ref path) = req.file_path {
        let prefix = resolve_scope_prefix(&scope, &uid);
        let key = format!("{prefix}{}", normalize_path(path));
        file_content_context.push_str(&format!("\nActive File Selected: {}\n", path));

        let ext = path.split('.').next_back().unwrap_or("").to_lowercase();
        if ext == "txt" || ext == "md" || ext == "json" || ext == "csv" {
            if let Ok(data) = drive.get_object(&bucket, &key).await {
                if data.len() < 10240 {
                    if let Ok(text) = String::from_utf8(data) {
                        file_content_context.push_str("File content preview:\n```\n");
                        file_content_context.push_str(&text.chars().take(2000).collect::<String>());
                        file_content_context.push_str("\n```\n");
                    }
                }
            }
        }
    }

    let prompt = format!(
        "You are the GeneralBots Drive AI Assistant. You have access to the user's file storage metadata and contents.\n\n\
        CONTEXT:\n\
        {}\n\
        {}\n\n\
        USER REQUEST: {}\n\n\
        Respond to the user request. Keep it helpful, precise, and professional. Translate/explain/classify files if requested.",
        files_context,
        file_content_context,
        req.message
    );

    let reply = provider
        .generate_simple(&prompt)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("AI generation error: {e}")))?;

    Ok(Json(AIChatResponse { reply }))
}

// ====== Public share links (token-based, revocable) ======
//
// Security model (enterprise):
// - A link is created only for a file the authenticated owner can access;
//   the storage key is captured at creation time (no scope/user input on
//   the public path).
// - share_token is a random 128-bit UUID — unguessable, never derived from
//   the file name.
// - The public endpoint resolves the token only; it cannot list, search, or
//   enumerate. Revoked and expired links behave exactly like unknown ones
//   (404), so existence is never leaked.
// - Files are served as attachment with nosniff and private caching so
//   revoked links stop working immediately and HTML/SVG payloads cannot run
//   in the browser origin.
