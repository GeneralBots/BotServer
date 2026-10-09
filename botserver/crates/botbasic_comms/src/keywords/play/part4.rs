use super::*;

pub(super) fn execute_play(
    state: &Arc<dyn BasicRuntime>,
    session_id: Uuid,
    source: &str,
    options: PlayOptions,
) -> Result<PlayResponse, String> {
    let content_type = detect_content_type(source);

    let source_url = resolve_source_url(state, session_id, source)?;

    let metadata = get_content_metadata(state, &source_url, &content_type);

    let player_id = Uuid::new_v4();

    let title = metadata
        .get("title")
        .cloned()
        .unwrap_or_else(|| extract_title_from_source(source));

    let response = PlayResponse {
        player_id,
        content_type: content_type.clone(),
        component: content_type.player_component().to_string(),
        source_url,
        title,
        options,
        metadata,
    };

    send_play_to_client(state, session_id, &response)?;

    info!(
        "Playing {:?} content: {source} for session {session_id}",
        response.content_type
    );

    Ok(response)
}
pub fn detect_content_type(source: &str) -> ContentType {
    if source.starts_with("http://") || source.starts_with("https://") {
        if source.contains("youtube.com")
            || source.contains("youtu.be")
            || source.contains("vimeo.com")
        {
            return ContentType::Video;
        }

        if source.contains("imgur.com")
            || source.contains("unsplash.com")
            || source.contains("pexels.com")
        {
            return ContentType::Image;
        }

        if let Some(path) = source.split('?').next() {
            if let Some(ext) = Path::new(path).extension() {
                return ContentType::from_extension(&ext.to_string_lossy());
            }
        }

        return ContentType::Iframe;
    }

    if let Some(ext) = Path::new(source).extension() {
        return ContentType::from_extension(&ext.to_string_lossy());
    }

    ContentType::Unknown
}
pub(super) fn resolve_source_url(_state: &Arc<dyn BasicRuntime>, session_id: Uuid, source: &str) -> Result<String, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        return Ok(source.to_string());
    }

    if source.starts_with('/') || source.contains(".gbdrive") {
        let file_url = format!(
            "/api/files/download?path={}&session={session_id}",
            urlencoding::encode(source)
        );
        return Ok(file_url);
    }

    let file_url = format!(
        "/api/files/download?path={}&session={session_id}",
        urlencoding::encode(source)
    );

    Ok(file_url)
}
pub(super) fn get_content_metadata(
    _state: &Arc<dyn BasicRuntime>,
    source_url: &str,
    content_type: &ContentType,
) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    metadata.insert("source".to_string(), source_url.to_string());
    metadata.insert("type".to_string(), format!("{content_type:?}"));

    match content_type {
        ContentType::Video | ContentType::Audio => {
            metadata.insert("player".to_string(), "html5".to_string());
        }
        ContentType::Image => {
            metadata.insert("viewer".to_string(), "lightbox".to_string());
        }
        ContentType::Pdf => {
            metadata.insert("viewer".to_string(), "pdfjs".to_string());
        }
        ContentType::Code => {
            metadata.insert("highlighter".to_string(), "prism".to_string());
        }
        ContentType::Presentation => {
            metadata.insert("viewer".to_string(), "revealjs".to_string());
        }
        ContentType::Spreadsheet => {
            metadata.insert("viewer".to_string(), "handsontable".to_string());
        }
        _ => {}
    }

    metadata
}
pub fn extract_title_from_source(source: &str) -> String {
    let path = source.split('?').next().unwrap_or(source);

    Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}
pub(super) fn send_play_to_client(
    state: &Arc<dyn BasicRuntime>,
    session_id: Uuid,
    response: &PlayResponse,
) -> Result<(), String> {
    let message = serde_json::json!({
        "type": "play",
        "data": response
    });

    let message_str =
        serde_json::to_string(&message).map_err(|e| format!("Failed to serialize: {e}"))?;

    state
        .web_adapter()
        .send_message_to_session(&session_id.to_string(), &message_str)
        .map_err(|e| format!("Failed to send to client: {e}"))?;

    Ok(())
}
pub(super) fn send_player_command(
    state: &Arc<dyn BasicRuntime>,
    session_id: Uuid,
    command: &str,
) -> Result<(), String> {
    let message = serde_json::json!({
        "type": "player_command",
        "command": command
    });

    let message_str =
        serde_json::to_string(&message).map_err(|e| format!("Failed to serialize: {e}"))?;

    let _ = state.web_adapter().send_message_to_session(
        &session_id.to_string(),
        &message_str,
    );

    Ok(())
}
