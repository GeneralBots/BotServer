use super::*;

pub(super) async fn send_web_message(
    state: Arc<AppState>,
    session_id: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let web_adapter = Arc::clone(&state.web_adapter);

    let response = botcore::shared::BotResponse {
        bot_id: "system".to_string(),
        user_id: session_id.to_string(),
        session_id: session_id.to_string(),
        channel: "web".to_string(),
        content: message.to_string(),
        message_type: MessageType::USER,
        stream_token: None,
        is_complete: true,
        suggestions: Vec::new(),
        switchers: Vec::new(),
        context_name: None,
        context_length: 0,
        context_max_length: 0,
            reasoning: String::new(),
    };

    web_adapter
        .send_message_to_session(session_id, &response.content)?;

    Ok(())
}
pub(super) async fn send_web_file(
    state: Arc<AppState>,
    session_id: &str,
    file_data: Vec<u8>,
    caption: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file_id = uuid::Uuid::new_v4().to_string();
    let file_url = format!("/api/files/{}", file_id);

    if let Some(redis_client) = &state.cache {
        let mut conn = redis_client.get_multiplexed_async_connection().await?;
        let file_key = format!("file:{}", file_id);

        redis::cmd("SET")
            .arg(&file_key)
            .arg(&file_data)
            .arg("EX")
            .arg(3600)
            .query_async::<()>(&mut conn)
            .await?;
    }

    let message = if caption.is_empty() {
        format!("[File: {}]", file_url)
    } else {
        format!("{}\n[File: {}]", caption, file_url)
    };

    send_web_message(state, session_id, &message).await
}
pub(super) fn send_email(
    state: Arc<AppState>,
    bot_id: uuid::Uuid,
    email: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "mail")]
    {
        use crate::email::EmailService;

        let email_service = EmailService::new(state);
        email_service.send_email(email, "Message from Bot", message, bot_id, None)?;
        Ok(())
    }

    #[cfg(not(feature = "mail"))]
    {
        let _ = ((state, bot_id), email, message);
        log::error!("Email feature not enabled");
        Err("Email feature not enabled".into())
    }
}
pub(super) fn send_email_attachment(
    state: Arc<AppState>,
    bot_id: uuid::Uuid,
    email: &str,
    file_data: Vec<u8>,
    caption: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "mail")]
    {
        use crate::email::EmailService;

        let email_service = EmailService::new(state);
        email_service.send_email_with_attachment(
            email,
            "File from Bot",
            caption,
            bot_id,
            file_data,
            "attachment",
        )?;
        Ok(())
    }

    #[cfg(not(feature = "mail"))]
    {
        let _ = ((state, bot_id), email, file_data, caption);
        log::error!("Email feature not enabled for attachments");
        Err("Email feature not enabled".into())
    }
}
pub(super) async fn get_teams_conversation_id(
    state: &Arc<AppState>,
    user_id: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(redis_client) = &state.cache {
        let mut conn = redis_client.get_multiplexed_async_connection().await?;
        let key = format!("teams_conversation:{}", user_id);

        if let Ok(conversation) = redis::cmd("GET")
            .arg(&key)
            .query_async::<String>(&mut conn)
            .await
        {
            return Ok(conversation);
        }
    }

    Ok(user_id.to_string())
}
