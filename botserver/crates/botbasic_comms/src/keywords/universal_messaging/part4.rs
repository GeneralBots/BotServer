use super::*;

pub(super) async fn send_file_to_recipient(
    state: Arc<AppState>,
    user: &UserSession,
    recipient: &str,
    file: Dynamic,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    send_file_with_caption_to_recipient(state, user, recipient, file, "").await
}
pub(super) async fn send_file_with_caption_to_recipient(
    state: Arc<AppState>,
    user: &UserSession,
    recipient: &str,
    file: Dynamic,
    caption: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (channel, recipient_id) = parse_recipient(state.clone(), recipient).await?;

    let file_data = if file.is_string() {
        let file_path = file.to_string();
        std::fs::read(&file_path)?
    } else {
        return Err("File must be a string path".into());
    };

    match channel.as_str() {
        "whatsapp" => {
            send_whatsapp_file(state, user, &recipient_id, file_data, caption).await?;
        }
        "instagram" => {
            #[cfg(feature = "drive")]
            send_instagram_file(state, user, &recipient_id, file_data, caption).await?;
            #[cfg(not(feature = "drive"))]
            return Err("Drive feature not enabled".into());
        }
        "teams" => {
            send_teams_file(state, user, &recipient_id, file_data, caption).await?;
        }
        "web" => {
            send_web_file(state, &recipient_id, file_data, caption).await?;
        }
        "email" => {
            send_email_attachment(state, user.bot_id, &recipient_id, file_data, caption)?;
        }
        _ => {
            return Err(format!("Unsupported channel for file sending: {}", channel).into());
        }
    }

    Ok(())
}
pub(super) async fn parse_recipient(
    state: Arc<AppState>,
    recipient: &str,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    if recipient.contains(':') {
        let parts: Vec<&str> = recipient.splitn(2, ':').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    if recipient.starts_with('+') || recipient.chars().all(|c| c.is_numeric()) {
        return Ok(("whatsapp".to_string(), recipient.to_string()));
    }

    if recipient.contains('@') {
        if recipient.ends_with("@teams.ms") || recipient.contains("@microsoft") {
            return Ok(("teams".to_string(), recipient.to_string()));
        }
        return Ok(("email".to_string(), recipient.to_string()));
    }

    if let Some(redis_client) = &state.cache {
        let mut conn = redis_client.get_multiplexed_async_connection().await?;
        let web_session_key = format!("web_session:{}", recipient);

        if redis::cmd("EXISTS")
            .arg(&web_session_key)
            .query_async::<bool>(&mut conn)
            .await?
        {
            return Ok(("web".to_string(), recipient.to_string()));
        }
    }

    Ok(("whatsapp".to_string(), recipient.to_string()))
}
pub(super) async fn send_to_specific_channel(
    state: Arc<AppState>,
    user: &UserSession,
    target: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    send_message_to_recipient(state, user, target, message).await
}
pub(super) async fn broadcast_message(
    state: Arc<AppState>,
    user: &UserSession,
    message: &str,
    recipients: Dynamic,
) -> Result<Dynamic, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    if recipients.is_array() {
        let recipient_list = match recipients.into_array() {
            Ok(arr) => arr,
            Err(_) => return Ok(Dynamic::from("[]")),
        };

        for recipient in recipient_list {
            let recipient_str = recipient.to_string();

            match send_message_to_recipient(state.clone(), user, &recipient_str, message).await {
                Ok(_) => {
                    results.push(json!({
                        "recipient": recipient_str,
                        "status": "sent"
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "recipient": recipient_str,
                        "status": "failed",
                        "error": e.to_string()
                    }));
                }
            }
        }
    }

    Ok(Dynamic::from(serde_json::to_string(&results)?))
}
pub(super) async fn send_whatsapp_file(
    state: Arc<AppState>,
    user: &UserSession,
    recipient: &str,
    file_data: Vec<u8>,
    caption: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use reqwest::Client;

    let _ = WhatsAppAdapter::new(&state, user.bot_id);

    let phone_number_id = "";
    let upload_url = format!("https://graph.facebook.com/v17.0/{}/media", phone_number_id);

    let client = Client::new();
    let form = reqwest::multipart::Form::new()
        .text("messaging_product", "whatsapp")
        .part("file", reqwest::multipart::Part::bytes(file_data));

    let access_token = "";
    let upload_response = client
        .post(&upload_url)
        .bearer_auth(access_token)
        .multipart(form)
        .send()
        .await?;

    if !upload_response.status().is_success() {
        return Err("Failed to upload file to WhatsApp".into());
    }

    let upload_result: serde_json::Value = upload_response.json().await?;
    let media_id = upload_result["id"].as_str().ok_or("No media ID returned")?;

    let send_url = format!(
        "https://graph.facebook.com/v17.0/{}/messages",
        phone_number_id
    );

    let payload = json!({
        "messaging_product": "whatsapp",
        "to": recipient,
        "type": "document",
        "document": {
            "id": media_id,
            "caption": caption
        }
    });

    client
        .post(&send_url)
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await?;

    Ok(())
}
#[cfg(feature = "drive")]
pub(super) async fn send_instagram_file(
    state: Arc<AppState>,
    user: &UserSession,
    recipient_id: &str,
    _file_data: Vec<u8>,
    caption: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let adapter = InstagramAdapter::new();

let file_key = format!("temp/instagram/{}_{}.bin", user.id, uuid::Uuid::new_v4());

let file_url = format!("https://s3.amazonaws.com/uploads/{}", file_key);

        // TODO: implement send_media_message for InstagramAdapter
            let _ = ((&adapter, recipient_id), &file_url);

        if !caption.is_empty() {
            adapter
                .send_instagram_message(recipient_id, caption)
                .await?;
        }

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        if let Some(s3) = &state.drive {
            let _ = s3.delete_object("uploads", &file_key).await;
        }
    });

    Ok(())
}
pub(super) async fn send_teams_file(
    state: Arc<AppState>,
    user: &UserSession,
    recipient_id: &str,
    file_data: Vec<u8>,
    caption: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = user;
    let _ = TeamsAdapter::new();

    let conversation_id = get_teams_conversation_id(&state, recipient_id).await?;

    let access_token = "";
    let service_url = "https://smba.trafficmanager.net/apis".to_string();
    let url = format!(
        "{}/v3/conversations/{}/activities",
        service_url.trim_end_matches('/'),
        conversation_id
    );

    use base64::{engine::general_purpose::STANDARD, Engine};
    let attachment = json!({
        "contentType": "application/octet-stream",
        "contentUrl": format!("data:application/octet-stream;base64,{}", STANDARD.encode(&file_data)),
        "name": "attachment"
    });

    let activity = json!({
        "type": "message",
        "text": caption,
        "from": {
            "id": "",
            "name": "Bot"
        },
        "conversation": {
            "id": conversation_id
        },
        "recipient": {
            "id": recipient_id
        },
        "attachments": [attachment]
    });

    let client = Client::new();
    client
        .post(&url)
        .bearer_auth(access_token)
        .json(&activity)
        .send()
        .await?;

    Ok(())
}
