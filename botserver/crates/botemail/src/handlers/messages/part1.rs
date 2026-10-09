use super::*;

pub(super) fn decrypt_password(encrypted: &str) -> Result<String, String> {
    general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|e| format!("Decryption failed: {e}"))
        .and_then(|bytes| String::from_utf8(bytes).map_err(|e| format!("UTF-8 conversion failed: {e}")))
}
#[cfg(feature = "mail")]
pub(super) fn parse_from_field(from: &str) -> (String, String) {
    if let Some(start) = from.find('<') {
        if let Some(end) = from.find('>') {
            let name = from[..start].trim().trim_matches('"').to_string();
            let email = from[start + 1..end].to_string();
            return (name, email);
        }
    }
    (String::new(), from.to_string())
}
#[cfg(feature = "mail")]
pub(super) fn format_email_time(date_str: &str) -> String {
    if date_str.is_empty() {
        return "Unknown".to_string();
    }
    date_str.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
}
#[cfg(feature = "mail")]
pub async fn list_emails(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ListEmailsRequest>,
) -> Result<Json<ApiResponse<Vec<EmailResponse>>>, EmailError> {
    let account_uuid = Uuid::parse_str(&request.account_id)
        .map_err(|_| EmailError("Invalid account ID".to_string()))?;

    let (imap_server, imap_port, username, encrypted_password) = {
        let pool = state.pool.clone();
        let account_info = tokio::task::spawn_blocking(move || {
            let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

            let result: ImapCredentialsRow = diesel::sql_query(
                "SELECT imap_server, imap_port, username, password_encrypted FROM user_email_accounts WHERE id = $1 AND is_active = true"
            )
            .bind::<diesel::sql_types::Uuid, _>(account_uuid)
            .get_result(&mut db_conn)
            .map_err(|e| format!("Account not found: {e}"))?;

            Ok::<_, String>(result)
        })
        .await
        .map_err(|e| EmailError(format!("Task join error: {e}")))?.map_err(EmailError)?;

        (account_info.imap_server, account_info.imap_port,
        account_info.username, account_info.password_encrypted)
    };

    let password = decrypt_password(&encrypted_password).map_err(EmailError)?;
    let client = imap::ClientBuilder::new(imap_server.as_str(), imap_port as u16)
        .connect()
        .map_err(|e| EmailError(format!("Failed to connect to IMAP: {e:?}")))?;

    let mut session = client
        .login(&username, &password)
        .map_err(|e| EmailError(format!("Login failed: {e:?}")))?;

    let folder = request.folder.unwrap_or_else(|| "INBOX".to_string());
    session.select(&folder)
        .map_err(|e| EmailError(format!("Failed to select folder: {e:?}")))?;

    let messages = session.search("ALL")
        .map_err(|e| EmailError(format!("Failed to search emails: {e:?}")))?;

    let mut email_list = Vec::new();
    let limit = request.limit.unwrap_or(50);
    let offset = request.offset.unwrap_or(0);

    let mut recent_messages: Vec<_> = messages.iter().copied().collect();
    recent_messages.sort_by(|a, b| b.cmp(a));
    let recent_messages: Vec<_> = recent_messages.into_iter().skip(offset).take(limit).collect();

    for seq in recent_messages {
        let fetch_result = session.fetch(seq.to_string(), "RFC822");
        let msgs = fetch_result.map_err(|e| EmailError(format!("Failed to fetch email: {e:?}")))?;

        for msg in msgs.iter() {
            let body = msg.body().ok_or_else(|| EmailError("No body found".to_string()))?;
            let parsed = parse_mail(body).map_err(|e| EmailError(format!("Failed to parse email: {e:?}")))?;

            let headers = parsed.get_headers();
            let subject = headers.get_first_value("Subject").unwrap_or_default();
            let from = headers.get_first_value("From").unwrap_or_default();
            let to = headers.get_first_value("To").unwrap_or_default();
            let date = headers.get_first_value("Date").unwrap_or_default();

            let body_text = parsed.subparts.iter()
                .find(|p| p.ctype.mimetype == "text/plain")
                .map_or_else(|| parsed.get_body().unwrap_or_default(), |bp| bp.get_body().unwrap_or_default());
            let body_html = parsed.subparts.iter()
                .find(|p| p.ctype.mimetype == "text/html")
                .map_or_else(String::new, |bp| bp.get_body().unwrap_or_default());

            let preview: String = body_text.lines().take(3).collect::<Vec<_>>().join(" ");
            let preview_truncated = if preview.len() > 150 { format!("{}...", &preview[..150]) } else { preview };

            let (from_name, from_email) = parse_from_field(&from);
            let has_attachments = parsed.subparts.iter().any(|p| {
                p.get_content_disposition().disposition == mailparse::DispositionType::Attachment
            });

            email_list.push(EmailResponse {
                id: seq.to_string(), from_name, from_email, to, subject,
                preview: preview_truncated,
                body: if body_html.is_empty() { body_text } else { body_html },
                date: format_email_time(&date), time: format_email_time(&date),
                read: false, folder: folder.clone(), has_attachments,
            });
        }
    }

    session.logout().ok();
    Ok(Json(ApiResponse { success: true, data: Some(email_list), message: None }))
}
#[cfg(not(feature = "mail"))]
pub async fn list_emails(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<ListEmailsRequest>,
) -> Result<Json<ApiResponse<Vec<EmailResponse>>>, EmailError> {
    Ok(Json(ApiResponse { success: false, data: Some(Vec::new()), message: Some("Mail feature not enabled".to_string()) }))
}
pub async fn send_email(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SendEmailRequest>,
) -> Result<Json<ApiResponse<()>>, EmailError> {
    let account_uuid = Uuid::parse_str(&request.account_id)
        .map_err(|_| EmailError("Invalid account ID".to_string()))?;

    let pool = state.pool.clone();
    let account_info = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

        let result: SmtpCredentialsRow = diesel::sql_query(
            "SELECT email, display_name, smtp_port, smtp_server, username, password_encrypted, auth_mode
             FROM user_email_accounts WHERE id = $1 AND is_active = true"
        )
            .bind::<diesel::sql_types::Uuid, _>(account_uuid)
            .get_result(&mut db_conn)
            .map_err(|e| format!("Account not found: {e}"))?;

        Ok::<_, String>(result)
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    let (from_email, display_name, smtp_port, smtp_server, username, encrypted_password, auth_mode) = (
        account_info.email, account_info.display_name,
        account_info.smtp_port, account_info.smtp_server,
        account_info.username, account_info.password_encrypted, account_info.auth_mode,
    );

    // Password accounts authenticate with the stored password. OAuth2 accounts
    // (Microsoft 365, Outlook.com, Gmail) carry a bearer token, and the
    // mechanism must be declared explicitly or the server rejects the AUTH.
    let (credentials, oauth_mechanism) = if auth_mode == "oauth2" {
        let credentials_pool = state.pool.clone();
        let token = tokio::task::spawn_blocking(move || {
            let mut db_conn = credentials_pool
                .get()
                .map_err(|e| format!("DB connection error: {e}"))?;
            crate::oauth::resolve_credentials(&mut db_conn, account_uuid).map(|c| c.secret)
        })
        .await
        .map_err(|e| EmailError(format!("Task join error: {e}")))?
        .map_err(EmailError)?;
        (Credentials::new(username.clone(), token), Some(Mechanism::Xoauth2))
    } else {
        let password = decrypt_password(&encrypted_password).map_err(EmailError)?;
        (Credentials::new(username.clone(), password), None)
    };

    let from_addr = if display_name.is_empty() {
        from_email.clone()
    } else {
        format!("{display_name} <{from_email}>")
    };

    let tracking_id = Uuid::new_v4();
    let pixel_enabled = (state.secrets_provider)("email-read-pixel").map(|v| v.to_lowercase() == "true").unwrap_or(false);

    let final_body = if pixel_enabled && request.is_html {
        tracking::inject_tracking_pixel(&request.body, &tracking_id.to_string(), &state)
    } else {
        request.body.clone()
    };

    let mut email_builder = Message::builder()
        .from(from_addr.parse().map_err(|e| EmailError(format!("Invalid from address: {e}")))?)
        .to(request.to.parse().map_err(|e| EmailError(format!("Invalid to address: {e}")))?)
        .subject(request.subject.clone());

    if let Some(ref cc) = request.cc {
        email_builder = email_builder.cc(cc.parse().map_err(|e| EmailError(format!("Invalid cc address: {e}")))?);
    }
    if let Some(ref bcc) = request.bcc {
        email_builder = email_builder.bcc(bcc.parse().map_err(|e| EmailError(format!("Invalid bcc address: {e}")))?);
    }

    let email = email_builder.body(final_body).map_err(|e| EmailError(format!("Failed to build email: {e}")))?;

    let mut transport = SmtpTransport::relay(&smtp_server)
        .map_err(|e| EmailError(format!("Failed to create SMTP transport: {e}")))?
        .port(u16::try_from(smtp_port).unwrap_or(587))
        .credentials(credentials);
    if let Some(mechanism) = oauth_mechanism {
        transport = transport.authentication(vec![mechanism]);
    }
    let mailer = transport.build();

    mailer.send(&email).map_err(|e| EmailError(format!("Failed to send email: {e}")))?;

    if pixel_enabled {
        let pool = state.pool.clone();
        let to_email = request.to.clone();
        let subject = request.subject.clone();
        let cc_clone = request.cc.clone();
        let bcc_clone = request.bcc.clone();
        let from_email_clone = from_email.clone();

        let _ = tokio::task::spawn_blocking(move || {
            tracking::save_email_tracking_record(
                pool,
                EmailTrackingParams {
                    tracking_id, account_id: account_uuid, bot_id: Uuid::nil(),
                    from_email: &from_email_clone, to_email: &to_email,
                    cc: cc_clone.as_deref(), bcc: bcc_clone.as_deref(), subject: &subject,
                },
            )
        })
        .await;
    }

    info!("Email sent successfully from account {account_uuid} with tracking_id {tracking_id}");

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        message: Some("Email sent successfully".to_string()),
    }))
}
pub async fn save_draft(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<SaveDraftRequest>,
) -> Result<Json<SaveDraftResponse>, EmailError> {
    let account_uuid = Uuid::parse_str(&request.account_id)
        .map_err(|_| EmailError("Invalid account ID".to_string()))?;

    let Ok(user_id) = extract_user_from_session(&headers) else {
        return Err(EmailError("Authentication required".to_string()));
    };
    let draft_id = Uuid::new_v4();

    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

        diesel::sql_query(
            "INSERT INTO email_drafts (id, user_id, account_id, to_address, cc_address, bcc_address, subject, body)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
            .bind::<diesel::sql_types::Uuid, _>(draft_id)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Uuid, _>(account_uuid)
            .bind::<diesel::sql_types::Text, _>(&request.to)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(request.cc.as_ref())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(request.bcc.as_ref())
            .bind::<diesel::sql_types::Text, _>(&request.subject)
            .bind::<diesel::sql_types::Text, _>(&request.body)
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to save draft: {e}"))?;

        Ok::<_, String>(())
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    Ok(Json(SaveDraftResponse {
        success: true,
        draft_id: Some(draft_id.to_string()),
        message: "Draft saved successfully".to_string(),
    }))
}
