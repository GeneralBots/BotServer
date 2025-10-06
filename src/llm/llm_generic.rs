use log::{error, info};

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

fn clean_request_body(body: &str) -> String {
    // Remove problematic parameters that might not be supported by all providers
    let re = Regex::new(r#","?\s*"(max_completion_tokens|parallel_tool_calls|top_p|frequency_penalty|presence_penalty)"\s*:\s*[^,}]*"#).unwrap();
    re.replace_all(body, "").to_string()
}

#[post("/v1/chat/completions")]
pub async fn generic_chat_completions(body: web::Bytes, _req: HttpRequest) -> Result<HttpResponse> {
    // Log raw POST data
    let body_str = std::str::from_utf8(&body).unwrap_or_default();
    info!("Original POST Data: {}", body_str);

    dotenv().ok();

    // Get environment variables
    let api_key = env::var("AI_KEY")
        .map_err(|_| actix_web::error::ErrorInternalServerError("AI_KEY not set."))?;
    let model = env::var("AI_LLM_MODEL")
        .map_err(|_| actix_web::error::ErrorInternalServerError("AI_LLM_MODEL not set."))?;
    let endpoint = env::var("AI_ENDPOINT")
        .map_err(|_| actix_web::error::ErrorInternalServerError("AI_ENDPOINT not set."))?;

    // Parse and modify the request body
    let mut json_value: serde_json::Value = serde_json::from_str(body_str)
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to parse JSON"))?;

    // Add model parameter
    if let Some(obj) = json_value.as_object_mut() {
        obj.insert("model".to_string(), serde_json::Value::String(model));
    }

    let modified_body_str = serde_json::to_string(&json_value)
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to serialize JSON"))?;

    info!("Modified POST Data: {}", modified_body_str);

    // Set up headers
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| actix_web::error::ErrorInternalServerError("Invalid API key format"))?,
    );
    headers.insert(
        "Content-Type",
        reqwest::header::HeaderValue::from_static("application/json"),
    );

    // Send request to the AI provider
    let client = Client::new();
    let response = client
        .post(&endpoint)
        .headers(headers)
        .body(modified_body_str)
        .send()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // Handle response
    let status = response.status();
    let raw_response = response
        .text()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    info!("Provider response status: {}", status);
    info!("Provider response body: {}", raw_response);

    // Convert response to OpenAI format if successful
    if status.is_success() {
        match convert_to_openai_format(&raw_response) {
            Ok(openai_response) => Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(openai_response)),
            Err(e) => {
                error!("Failed to convert response format: {}", e);
                // Return the original response if conversion fails
                Ok(HttpResponse::Ok()
                    .content_type("application/json")
                    .body(raw_response))
            }
        }
    } else {
        // Return error as-is
        let actix_status = actix_web::http::StatusCode::from_u16(status.as_u16())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

        Ok(HttpResponse::build(actix_status)
            .content_type("application/json")
            .body(raw_response))
    }
}

/// Converts provider response to OpenAI-compatible format
fn convert_to_openai_format(provider_response: &str) -> Result<String, Box<dyn std::error::Error>> {
    #[derive(serde::Deserialize)]
    struct ProviderChoice {
        message: ProviderMessage,
        #[serde(default)]
        finish_reason: Option<String>,
    }

    #[derive(serde::Deserialize)]
    struct ProviderMessage {
        role: Option<String>,
        content: String,
    }

    #[derive(serde::Deserialize)]
    struct ProviderResponse {
        id: Option<String>,
        object: Option<String>,
        created: Option<u64>,
        model: Option<String>,
        choices: Vec<ProviderChoice>,
        usage: Option<ProviderUsage>,
    }

    #[derive(serde::Deserialize, Default)]
    struct ProviderUsage {
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
        total_tokens: Option<u32>,
    }

    #[derive(serde::Serialize)]
    struct OpenAIResponse {
        id: String,
        object: String,
        created: u64,
        model: String,
        choices: Vec<OpenAIChoice>,
        usage: OpenAIUsage,
    }

    #[derive(serde::Serialize)]
    struct OpenAIChoice {
        index: u32,
        message: OpenAIMessage,
        finish_reason: String,
    }

    #[derive(serde::Serialize)]
    struct OpenAIMessage {
        role: String,
        content: String,
    }

    #[derive(serde::Serialize)]
    struct OpenAIUsage {
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    }

    // Parse the provider response
    let provider: ProviderResponse = serde_json::from_str(provider_response)?;

    // Extract content from the first choice
    let first_choice = provider.choices.get(0).ok_or("No choices in response")?;
    let content = first_choice.message.content.clone();
    let role = first_choice
        .message
        .role
        .clone()
        .unwrap_or_else(|| "assistant".to_string());

    // Calculate token usage
    let usage = provider.usage.unwrap_or_default();
    let prompt_tokens = usage.prompt_tokens.unwrap_or(0);
    let completion_tokens = usage
        .completion_tokens
        .unwrap_or_else(|| content.split_whitespace().count() as u32);
    let total_tokens = usage
        .total_tokens
        .unwrap_or(prompt_tokens + completion_tokens);

    let openai_response = OpenAIResponse {
        id: provider
            .id
            .unwrap_or_else(|| format!("chatcmpl-{}", uuid::Uuid::new_v4().simple())),
        object: provider
            .object
            .unwrap_or_else(|| "chat.completion".to_string()),
        created: provider.created.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        }),
        model: provider.model.unwrap_or_else(|| "llama".to_string()),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage { role, content },
            finish_reason: first_choice
                .finish_reason
                .clone()
                .unwrap_or_else(|| "stop".to_string()),
        }],
        usage: OpenAIUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        },
    };

    serde_json::to_string(&openai_response).map_err(|e| e.into())
}

// Default implementation for ProviderUsage
