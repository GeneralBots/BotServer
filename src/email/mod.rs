use actix_web::{post, web, HttpResponse, Result};
use lettre::{
    message::header::ContentType, 
    transport::smtp::authentication::Credentials, 
    Message, 
    SmtpTransport, 
    Transport
};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Clone)]
pub struct EmailConfig {
    pub from: String,
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

async fn send_email_impl(
    config: &EmailConfig,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let email = Message::builder()
        .from(config.from.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())?;

    let creds = Credentials::new(config.username.clone(), config.password.clone());

    let mailer = SmtpTransport::relay(&config.server)?
        .port(config.port)
        .credentials(creds)
        .build();

    match mailer.send(&email) {
        Ok(_) => {
            info!("Email sent to {}", to);
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to send email: {}", e);
            Err(Box::new(e))
        }
    }
}

#[post("/email/send")]
pub async fn send_email(
    config: web::Data<crate::config::AppConfig>,
    payload: web::Json<EmailRequest>,
) -> Result<HttpResponse> {
    let email_request = payload.into_inner();
    
    match send_email_impl(&config.email, &email_request.to, &email_request.subject, &email_request.body).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"status": "sent"}))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
    }
}

#[post("/email/test")]
pub async fn test_email(
    config: web::Data<crate::config::AppConfig>,
) -> Result<HttpResponse> {
    match send_email_impl(&config.email, &config.email.from, "Test Email", "This is a test email from BotServer").await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"status": "test_sent"}))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})))
    }
}
