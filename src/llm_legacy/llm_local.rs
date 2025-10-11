use dotenvy::dotenv;
use log::{error, info, warn};
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct LocalChatRequest {
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

#[derive(Debug, Deserialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: String,
}

#[derive(Debug, Serialize)]
pub struct LocalChatResponse {
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

#[derive(Debug, Serialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: u32,
}

pub async fn ensure_llama_servers_running() -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking if local LLM servers are running...");
    
    // For now, just log that we would start servers
    info!("Local LLM servers would be started here");
    
    Ok(())
}

pub async fn chat_completions_local(
    payload: web::Json<LocalChatRequest>,
) -> Result<HttpResponse> {
    dotenv().ok();

    info!("Received local chat request for model: {}", payload.model);

    // Mock response for local LLM
    let response = LocalChatResponse {
        id: "local-chat-123".to_string(),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: payload.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "This is a mock response from the local LLM. In a real implementation, this would connect to a local model like Llama or Mistral.".to_string(),
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Usage {
            prompt_tokens: 15,
            completion_tokens: 25,
            total_tokens: 40,
        },
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn embeddings_local(
    payload: web::Json<EmbeddingRequest>,
) -> Result<HttpResponse> {
    dotenv().ok();

    info!("Received local embedding request for model: {}", payload.model);

    // Mock embedding response
    let response = EmbeddingResponse {
        object: "list".to_string(),
        data: vec![EmbeddingData {
            object: "embedding".to_string(),
            embedding: vec![0.1; 768], // Mock embedding vector
            index: 0,
        }],
        model: payload.model.clone(),
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 0,
            total_tokens: 10,
        },
    };

    Ok(HttpResponse::Ok().json(response))
}
