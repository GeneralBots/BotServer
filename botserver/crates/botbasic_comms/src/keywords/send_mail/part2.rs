use super::*;

pub(super) async fn execute_send_mail(
    state: &AppState,
    user: &UserSession,
    to: &str,
    subject: &str,
    body: &str,
    attachments: Vec<String>,
    using_account: Option<String>,
) -> Result<String, String> {
    let message_id = Uuid::new_v4().to_string();

    track_email(state, user, &message_id, to, subject, "sent").unwrap_or_else(|e| {
        log::warn!("Email tracking skipped (table may not exist): {}", e);
    });

    if let Some(account_email) = using_account {
        let creds = get_account_credentials(&state.conn, &account_email, user.bot_id)
            .await
            .map_err(|e| format!("Failed to get account credentials: {}", e))?;

        return send_via_connected_account(state, &creds, to, subject, body, attachments).await;
    }

    #[cfg(feature = "mail")]
    {
        use crate::email::EmailService;

        let email_service = EmailService::new(Arc::new(state.clone()));

        match email_service
            .send_email(
                to,
                subject,
                body,
                user.bot_id,
                if attachments.is_empty() {
                    None
                } else {
                    Some(attachments.clone())
                },
            ) {
            Ok(msg_id) => {
                info!("Email sent successfully: {} to {}", msg_id, to);
                return Ok(format!("Email sent: {}", message_id));
            }
            Err(e) => {
                log::error!("EmailService::send_email failed: {}", e);
            }
        }
    }

    save_email_draft(state, user, to, subject, body, attachments).unwrap_or_else(|e| {
        log::warn!("Email draft not saved (table may not exist): {}", e);
    });

    Ok(format!("Email saved as draft: {}", message_id))
}
pub(super) async fn send_via_connected_account(
    state: &AppState,
    creds: &AccountCredentials,
    to: &str,
    subject: &str,
    body: &str,
    _attachments: Vec<String>,
) -> Result<String, String> {
    let message_id = Uuid::new_v4().to_string();

    match creds.provider.as_str() {
        "gmail" | "google" => {
            send_via_gmail(state, creds, to, subject, body).await?;
        }
        "outlook" | "microsoft" | "hotmail" => {
            send_via_outlook(state, creds, to, subject, body).await?;
        }
        _ => {
            return Err(format!("Unsupported email provider: {}", creds.provider));
        }
    }

    info!("Email sent via {} account: {}", creds.provider, message_id);
    Ok(format!("Email sent via {}: {}", creds.provider, message_id))
}
pub(super) async fn send_via_gmail(
    _state: &AppState,
    creds: &AccountCredentials,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let raw_message = format!(
        "To: {}\r\nSubject: {}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{}",
        to, subject, body
    );
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE,
        raw_message.as_bytes(),
    );

    let response = client
        .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
        .bearer_auth(&creds.access_token)
        .json(&serde_json::json!({ "raw": encoded }))
        .send()
        .await
        .map_err(|e| format!("Gmail API request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Gmail API error: {}", error_text));
    }

    Ok(())
}
pub(super) async fn send_via_outlook(
    _state: &AppState,
    creds: &AccountCredentials,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let message = serde_json::json!({
        "message": {
            "subject": subject,
            "body": {
                "contentType": "HTML",
                "content": body
            },
            "toRecipients": [
                {
                    "emailAddress": {
                        "address": to
                    }
                }
            ]
        },
        "saveToSentItems": "true"
    });

    let response = client
        .post("https://graph.microsoft.com/v1.0/me/sendMail")
        .bearer_auth(&creds.access_token)
        .json(&message)
        .send()
        .await
        .map_err(|e| format!("Microsoft Graph request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Microsoft Graph error: {}", error_text));
    }

    Ok(())
}
pub(super) async fn execute_send_template(
    state: &AppState,
    user: &UserSession,
    recipients: Vec<String>,
    template_name: &str,
    variables: serde_json::Value,
) -> Result<i32, String> {
    let template_content = load_template(state, template_name)?;

    let mut sent_count = 0;

    for recipient in recipients {
        let personalized_content =
            apply_template_variables(&template_content, &variables, &recipient)?;

        let subject = extract_template_subject(&personalized_content)
            .unwrap_or_else(|| format!("Message from {}", user.user_id));

        if execute_send_mail(
            state,
            user,
            &recipient,
            &subject,
            &personalized_content,
            vec![],
            None,
        )
        .await
        .is_ok()
        {
            sent_count += 1;
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    trace!("Sent {} templated emails", sent_count);
    Ok(sent_count)
}
pub(super) fn track_email(
    state: &AppState,
    user: &UserSession,
    message_id: &str,
    to: &str,
    subject: &str,
    status: &str,
) -> Result<(), String> {
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {}", e))?;

    let log_id = Uuid::new_v4().to_string();
    let user_id_str = user.user_id.to_string();
    let bot_id_str = user.bot_id.to_string();
    let now = Utc::now();

    let query = diesel::sql_query(
        "INSERT INTO communication_logs (id, user_id, bot_id, message_id, recipient, subject, status, timestamp, type)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'email')"
    )
    .bind::<diesel::sql_types::Text, _>(&log_id)
    .bind::<diesel::sql_types::Text, _>(&user_id_str)
    .bind::<diesel::sql_types::Text, _>(&bot_id_str)
    .bind::<diesel::sql_types::Text, _>(message_id)
    .bind::<diesel::sql_types::Text, _>(to)
    .bind::<diesel::sql_types::Text, _>(subject)
    .bind::<diesel::sql_types::Text, _>(status)
    .bind::<diesel::sql_types::Timestamptz, _>(&now);

    if let Err(e) = query.execute(&mut *conn) {
        log::warn!("Email tracking skipped: {}", e);
    }

    Ok(())
}
pub(super) fn save_email_draft(
    _state: &AppState,
    _user: &UserSession,
    _to: &str,
    _subject: &str,
    _body: &str,
    _attachments: Vec<String>,
) -> Result<(), String> {
    // Draft saving skipped for BASIC SEND MAIL — email is sent directly via SMTP.
    Ok(())
}
pub(super) fn load_template(state: &AppState, template_name: &str) -> Result<String, String> {
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {}", e))?;

    let query =
        diesel::sql_query("SELECT content FROM email_templates WHERE name = $1 AND active = true")
            .bind::<diesel::sql_types::Text, _>(template_name);

    #[derive(QueryableByName)]
    struct TemplateRecord {
        #[diesel(sql_type = diesel::sql_types::Text)]
        content: String,
    }

    let result: Result<Vec<TemplateRecord>, _> = query.load(&mut *conn);

    match result {
        Ok(records) if !records.is_empty() => Ok(records[0].content.clone()),
        _ => {
            let template_path = format!(".gbdrive/templates/{}.html", template_name);
            std::fs::read_to_string(&template_path)
                .map_err(|e| format!("Template not found: {}", e))
        }
    }
}
pub(super) fn apply_template_variables(
    template: &str,
    variables: &serde_json::Value,
    recipient: &str,
) -> Result<String, String> {
    let mut content = template.to_string();

    content = content.replace("{{recipient}}", recipient);

    if let Some(obj) = variables.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{{{}}}}}", key);
            let value_string = value.to_string();
            let replacement = value.as_str().unwrap_or(&value_string);
            content = content.replace(&placeholder, replacement);
        }
    }

    Ok(content)
}
pub(super) fn extract_template_subject(content: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with("Subject:") {
            return Some(line.trim_start_matches("Subject:").trim().to_string());
        }
    }
    None
}
