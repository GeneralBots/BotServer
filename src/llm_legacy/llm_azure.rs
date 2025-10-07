use log::info;

use actix_web::{post, web, HttpRequest, HttpResponse, Result};
use dotenv::dotenv;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

// OpenAI-compatible request/response structures
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: ChatMessage,
    finish_reason: String,
}

#[post("/azure/v1/chat/completions")]
async fn chat_completions(body: web::Bytes, _req: HttpRequest) -> Result<HttpResponse> {
    // Always log raw POST data
    if let Ok(body_str) = std::str::from_utf8(&body) {
        info!("POST Data: {}", body_str);
    } else {
        info!("POST Data (binary): {:?}", body);
    }

    dotenv().ok();

    // Environment variables
    let azure_endpoint = env::var("AI_ENDPOINT")
        .map_err(|_| actix_web::error::ErrorInternalServerError("AI_ENDPOINT not set."))?;
    let azure_key = env::var("AI_KEY")
        .map_err(|_| actix_web::error::ErrorInternalServerError("AI_KEY not set."))?;
    let deployment_name = env::var("AI_LLM_MODEL")
        .map_err(|_| actix_web::error::ErrorInternalServerError("AI_LLM_MODEL not set."))?;

    // Construct Azure OpenAI URL
    let url = format!(
        "{}/openai/deployments/{}/chat/completions?api-version=2025-01-01-preview",
        azure_endpoint, deployment_name
    );

    // Forward headers
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "api-key",
        reqwest::header::HeaderValue::from_str(&azure_key)
            .map_err(|_| actix_web::error::ErrorInternalServerError("Invalid Azure key"))?,
    );
    headers.insert(
        "Content-Type",
        reqwest::header::HeaderValue::from_static("application/json"),
    );

    let body_str = std::str::from_utf8(&body).unwrap_or("");
    info!("Original POST Data: {}", body_str);

    // Remove the problematic params
    let re =
        Regex::new(r#","?\s*"(max_completion_tokens|parallel_tool_calls)"\s*:\s*[^,}]*"#).unwrap();
    let cleaned = re.replace_all(body_str, "");
    let cleaned_body = web::Bytes::from(cleaned.to_string());

    info!("Cleaned POST Data: {}", cleaned);

    // Send request to Azure
    let client = Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .body(cleaned_body)
        .send()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // Handle response based on status
    let status = response.status();
    let raw_response = response
        .text()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // Log the raw response
    info!("Raw Azure response: {}", raw_response);

    if status.is_success() {
        Ok(HttpResponse::Ok().body(raw_response))
    } else {
        // Handle error responses properly
        let actix_status = actix_web::http::StatusCode::from_u16(status.as_u16())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

        Ok(HttpResponse::build(actix_status).body(raw_response))
    }
}
