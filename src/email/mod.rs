use crate::{config::EmailConfig, shared::state::AppState};
use log::info;

use actix_web::error::ErrorInternalServerError;
use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse, Result};
use lettre::{transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use serde::Serialize;

use imap::types::Seq;
use mailparse::{parse_mail, MailHeaderMap}; // Added MailHeaderMap import

#[derive(Debug, Serialize)]
pub struct EmailResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub subject: String,
    pub text: String,
    date: String,
    read: bool,
    labels: Vec<String>,
}

async fn internal_send_email(config: &EmailConfig, to: &str, subject: &str, body: &str) {
    let email = Message::builder()
        .from(config.from.parse().unwrap())
        .to(to.parse().unwrap())
        .subject(subject)
        .body(body.to_string())
        .unwrap();

    let creds = Credentials::new(config.username.clone(), config.password.clone());

    SmtpTransport::relay(&config.server)
        .unwrap()
        .port(config.port)
        .credentials(creds)
        .build()
        .send(&email)
        .unwrap();
}

#[actix_web::get("/emails/list")]
pub async fn list_emails(
    state: web::Data<AppState>,
) -> Result<web::Json<Vec<EmailResponse>>, actix_web::Error> {
    let _config = state
        .config
        .as_ref()
        .ok_or_else(|| ErrorInternalServerError("Configuration not available"))?;

    // Establish connection
    let tls = native_tls::TlsConnector::builder().build().map_err(|e| {
        ErrorInternalServerError(format!("Failed to create TLS connector: {:?}", e))
    })?;

    let client = imap::connect(
        (_config.email.server.as_str(), 993),
        _config.email.server.as_str(),
        &tls,
    )
    .map_err(|e| ErrorInternalServerError(format!("Failed to connect to IMAP: {:?}", e)))?;

    // Login
    let mut session = client
        .login(&_config.email.username, &_config.email.password)
        .map_err(|e| ErrorInternalServerError(format!("Login failed: {:?}", e)))?;

    // Select INBOX
    session
        .select("INBOX")
        .map_err(|e| ErrorInternalServerError(format!("Failed to select INBOX: {:?}", e)))?;

    // Search for all messages
    let messages = session
        .search("ALL")
        .map_err(|e| ErrorInternalServerError(format!("Failed to search emails: {:?}", e)))?;

    let mut email_list = Vec::new();

    // Get last 20 messages
    let recent_messages: Vec<_> = messages.iter().cloned().collect(); // Collect items into a Vec
    let recent_messages: Vec<Seq> = recent_messages.into_iter().rev().take(20).collect(); // Now you can reverse and take the last 20
    for seq in recent_messages {
        // Fetch the entire message (headers + body)
        let fetch_result = session.fetch(seq.to_string(), "RFC822");
        let messages = fetch_result
            .map_err(|e| ErrorInternalServerError(format!("Failed to fetch email: {:?}", e)))?;

        for msg in messages.iter() {
            let body = msg
                .body()
                .ok_or_else(|| ErrorInternalServerError("No body found"))?;

            // Parse the complete email message
            let parsed = parse_mail(body)
                .map_err(|e| ErrorInternalServerError(format!("Failed to parse email: {:?}", e)))?;

            // Extract headers
            let headers = parsed.get_headers();
            let subject = headers.get_first_value("Subject").unwrap_or_default();
            let from = headers.get_first_value("From").unwrap_or_default();
            let date = headers.get_first_value("Date").unwrap_or_default();

            // Extract body text (handles both simple and multipart emails)
            let body_text = if let Some(body_part) = parsed
                .subparts
                .iter()
                .find(|p| p.ctype.mimetype == "text/plain")
            {
                body_part.get_body().unwrap_or_default()
            } else {
                parsed.get_body().unwrap_or_default()
            };

            // Create preview
            let preview = body_text.lines().take(3).collect::<Vec<_>>().join(" ");
            let preview_truncated = if preview.len() > 150 {
                format!("{}...", &preview[..150])
            } else {
                preview
            };

            // Parse From field
            let (from_name, from_email) = parse_from_field(&from);

            email_list.push(EmailResponse {
                id: seq.to_string(),
                name: from_name,
                email: from_email,
                subject: if subject.is_empty() {
                    "(No Subject)".to_string()
                } else {
                    subject
                },
                text: preview_truncated,
                date: if date.is_empty() {
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    date
                },
                read: false,
                labels: Vec::new(),
            });
        }
    }

    session
        .logout()
        .map_err(|e| ErrorInternalServerError(format!("Failed to logout: {:?}", e)))?;

    Ok(web::Json(email_list))
}

// Helper function to parse From field
fn parse_from_field(from: &str) -> (String, String) {
    if let Some(start) = from.find('<') {
        if let Some(end) = from.find('>') {
            let email = from[start + 1..end].trim().to_string();
            let name = from[..start].trim().trim_matches('"').to_string();
            return (name, email);
        }
    }
    ("Unknown".to_string(), from.to_string())
}

#[derive(serde::Deserialize)]
pub struct SaveDraftRequest {
    pub to: String,
    pub subject: String,
    pub cc: Option<String>,
    pub text: String,
}

#[derive(serde::Serialize)]
pub struct SaveDraftResponse {
    pub success: bool,
    pub message: String,
    pub draft_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct GetLatestEmailRequest {
    pub from_email: String,
}

#[derive(serde::Serialize)]
pub struct LatestEmailResponse {
    pub success: bool,
    pub email_text: Option<String>,
    pub message: String,
}

#[actix_web::post("/emails/save_draft")]
pub async fn save_draft(
    state: web::Data<AppState>,
    draft_data: web::Json<SaveDraftRequest>,
) -> Result<web::Json<SaveDraftResponse>, actix_web::Error> {
    let config = state
        .config
        .as_ref()
        .ok_or_else(|| ErrorInternalServerError("Configuration not available"))?;

    match save_email_draft(&config.email, &draft_data).await {
        Ok(draft_id) => Ok(web::Json(SaveDraftResponse {
            success: true,
            message: "Draft saved successfully".to_string(),
            draft_id: Some(draft_id),
        })),
        Err(e) => Ok(web::Json(SaveDraftResponse {
            success: false,
            message: format!("Failed to save draft: {}", e),
            draft_id: None,
        })),
    }
}

pub async fn save_email_draft(
    email_config: &EmailConfig,
    draft_data: &SaveDraftRequest,
) -> Result<String, Box<dyn std::error::Error>> {
    // Establish connection
    let tls = native_tls::TlsConnector::builder().build()?;
    let client = imap::connect(
        (email_config.server.as_str(), 993),
        email_config.server.as_str(),
        &tls,
    )?;

    // Login
    let mut session = client
        .login(&email_config.username, &email_config.password)
        .map_err(|e| format!("Login failed: {:?}", e))?;

    // Select or create Drafts folder
    if session.select("Drafts").is_err() {
        // Try to create Drafts folder if it doesn't exist
        session.create("Drafts")?;
        session.select("Drafts")?;
    }

    // Create email message
    let cc_header = draft_data
        .cc
        .as_deref()
        .filter(|cc| !cc.is_empty())
        .map(|cc| format!("Cc: {}\r\n", cc))
        .unwrap_or_default();
    let email_message = format!(
        "From: {}\r\nTo: {}\r\n{}Subject: {}\r\nDate: {}\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n{}",
        email_config.username,
        draft_data.to,
        cc_header,
        draft_data.subject,
        chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S +0000"),
        draft_data.text
    );

    // Append to Drafts folder
    session.append("Drafts", &email_message)?;

    session.logout()?;

    Ok(chrono::Utc::now().timestamp().to_string())
}

async fn fetch_latest_email_from_sender(
    email_config: &EmailConfig,
    from_email: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Establish connection
    let tls = native_tls::TlsConnector::builder().build()?;
    let client = imap::connect(
        (email_config.server.as_str(), 993),
        email_config.server.as_str(),
        &tls,
    )?;

    // Login
    let mut session = client
        .login(&email_config.username, &email_config.password)
        .map_err(|e| format!("Login failed: {:?}", e))?;

    // Try to select Archive folder first, then fall back to INBOX
    if session.select("Archive").is_err() {
        session.select("INBOX")?;
    }

    // Search for emails from the specified sender
    let search_query = format!("FROM \"{}\"", from_email);
    let messages = session.search(&search_query)?;

    if messages.is_empty() {
        session.logout()?;
        return Err(format!("No emails found from {}", from_email).into());
    }

    // Get the latest message (highest sequence number)
    let latest_seq = messages.iter().max().unwrap();

    // Fetch the entire message
    let messages = session.fetch(latest_seq.to_string(), "RFC822")?;

    let mut email_text = String::new();

    for msg in messages.iter() {
        let body = msg.body().ok_or("No body found in email")?;

        // Parse the complete email message
        let parsed = parse_mail(body)?;

        // Extract headers
        let headers = parsed.get_headers();
        let subject = headers.get_first_value("Subject").unwrap_or_default();
        let from = headers.get_first_value("From").unwrap_or_default();
        let date = headers.get_first_value("Date").unwrap_or_default();
        let to = headers.get_first_value("To").unwrap_or_default();

        // Extract body text
        let body_text = if let Some(body_part) = parsed
            .subparts
            .iter()
            .find(|p| p.ctype.mimetype == "text/plain")
        {
            body_part.get_body().unwrap_or_default()
        } else {
            parsed.get_body().unwrap_or_default()
        };

        // Format the email text ready for reply with headers
        email_text = format!(
            "--- Original Message ---\nFrom: {}\nTo: {}\nDate: {}\nSubject: {}\n\n{}\n\n--- Reply Above This Line ---\n\n",
            from, to, date, subject, body_text
        );

        break; // We only want the first (and should be only) message
    }

    session.logout()?;

    if email_text.is_empty() {
        Err("Failed to extract email content".into())
    } else {
        Ok(email_text)
    }
}

#[actix_web::post("/emails/get_latest_from")]
pub async fn get_latest_email_from(
    state: web::Data<AppState>,
    request: web::Json<GetLatestEmailRequest>,
) -> Result<web::Json<LatestEmailResponse>, actix_web::Error> {
    let config = state
        .config
        .as_ref()
        .ok_or_else(|| ErrorInternalServerError("Configuration not available"))?;

    match fetch_latest_email_from_sender(&config.email, &request.from_email).await {
        Ok(email_text) => Ok(web::Json(LatestEmailResponse {
            success: true,
            email_text: Some(email_text),
            message: "Latest email retrieved successfully".to_string(),
        })),
        Err(e) => {
            if e.to_string().contains("No emails found") {
                Ok(web::Json(LatestEmailResponse {
                    success: false,
                    email_text: None,
                    message: e.to_string(),
                }))
            } else {
                Err(ErrorInternalServerError(e))
            }
        }
    }
}

pub async fn fetch_latest_sent_to(
    email_config: &EmailConfig,
    to_email: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Establish connection
    let tls = native_tls::TlsConnector::builder().build()?;
    let client = imap::connect(
        (email_config.server.as_str(), 993),
        email_config.server.as_str(),
        &tls,
    )?;

    // Login
    let mut session = client
        .login(&email_config.username, &email_config.password)
        .map_err(|e| format!("Login failed: {:?}", e))?;

    // Try to select Archive folder first, then fall back to INBOX
    if session.select("Sent").is_err() {
        session.select("Sent Items")?;
    }

    // Search for emails from the specified sender
    let search_query = format!("TO \"{}\"", to_email);
    let messages = session.search(&search_query)?;

    if messages.is_empty() {
        session.logout()?;
        return Err(format!("No emails found to {}", to_email).into());
    }

    // Get the latest message (highest sequence number)
    let latest_seq = messages.iter().max().unwrap();

    // Fetch the entire message
    let messages = session.fetch(latest_seq.to_string(), "RFC822")?;

    let mut email_text = String::new();

    for msg in messages.iter() {
        let body = msg.body().ok_or("No body found in email")?;

        // Parse the complete email message
        let parsed = parse_mail(body)?;

        // Extract headers
        let headers = parsed.get_headers();
        let subject = headers.get_first_value("Subject").unwrap_or_default();
        let from = headers.get_first_value("From").unwrap_or_default();
        let date = headers.get_first_value("Date").unwrap_or_default();
        let to = headers.get_first_value("To").unwrap_or_default();

        if !to
            .trim()
            .to_lowercase()
            .contains(&to_email.trim().to_lowercase())
        {
            continue;
        }
        // Extract body text (handles both simple and multipart emails) - SAME AS LIST_EMAILS
        let body_text = if let Some(body_part) = parsed
            .subparts
            .iter()
            .find(|p| p.ctype.mimetype == "text/plain")
        {
            body_part.get_body().unwrap_or_default()
        } else {
            parsed.get_body().unwrap_or_default()
        };

        // Only format if we have actual content
        if !body_text.trim().is_empty() && body_text != "No readable content found" {
            // Format the email text ready for reply with headers
            email_text = format!(
                "--- Original Message ---\nFrom: {}\nTo: {}\nDate: {}\nSubject: {}\n\n{}\n\n--- Reply Above This Line ---\n\n",
                from, to, date, subject, body_text.trim()
            );
        } else {
            // Still provide headers even if body is empty
            email_text = format!(
                "--- Original Message ---\nFrom: {}\nTo: {}\nDate: {}\nSubject: {}\n\n[No readable content]\n\n--- Reply Above This Line ---\n\n",
                from, to, date, subject
            );
        }

        break; // We only want the first (and should be only) message
    }

    session.logout()?;

    // Always return something, even if it's just headers
    if email_text.is_empty() {
        Err("Failed to extract email content".into())
    } else {
        Ok(email_text)
    }
}

#[actix_web::post("/emails/send")]
pub async fn send_email(
    payload: web::Json<(String, String, String)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let (to, subject, body) = payload.into_inner();

    info!("To: {}", to);
    info!("Subject: {}", subject);
    info!("Body: {}", body);

    // Send via SMTP
    internal_send_email(&state.config.clone().unwrap().email, &to, &subject, &body).await;

    Ok(HttpResponse::Ok().finish())
}

#[actix_web::get("/campaigns/{campaign_id}/click/{email}")]
pub async fn save_click(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let (campaign_id, email) = path.into_inner();
    let _ = sqlx::query("INSERT INTO public.clicks (campaign_id, email, updated_at) VALUES ($1, $2, NOW()) ON CONFLICT (campaign_id, email) DO UPDATE SET updated_at = NOW()")
        .bind(campaign_id)
        .bind(email)
        .execute(state.db.as_ref().unwrap())
        .await;

    let pixel = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG header
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimension
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, // RGBA
        0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
        0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, // data
        0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, // CRC
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
        0xAE, 0x42, 0x60, 0x82,
    ]; // EOF

    // At the end of your save_click function:
    HttpResponse::Ok()
        .content_type(ContentType::png())
        .body(pixel.to_vec()) // Using slicing to pass a reference
}

#[actix_web::get("/campaigns/{campaign_id}/emails")]
pub async fn get_emails(path: web::Path<String>, state: web::Data<AppState>) -> String {
    let campaign_id = path.into_inner();
    let rows = sqlx::query_scalar::<_, String>("SELECT email FROM clicks WHERE campaign_id = $1")
        .bind(campaign_id)
        .fetch_all(state.db.as_ref().unwrap())
        .await
        .unwrap_or_default();
    rows.join(",")
}
