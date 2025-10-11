use actix_web::{web, HttpResponse, Result};
use dotenvy::dotenv;
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GenericChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct GenericChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    pub endpoint: String,
    pub api_key: String,
    pub models: Vec<String>,
}

pub async fn generic_chat_completions(
    payload: web::Json<GenericChatRequest>,
) -> Result<HttpResponse> {
    dotenv().ok();

    info!("Received generic chat request for model: {}", payload.model);

    // For now, return a mock response
    let response = GenericChatResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1677652288,
        model: payload.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "This is a mock response from the generic LLM endpoint.".to_string(),
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        },
    };

    Ok(HttpResponse::Ok().json(response))
}
