use dotenvy::dotenv;
use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct AzureOpenAIConfig {
    pub endpoint: String,
    pub api_key: String,
    pub api_version: String,
    pub deployment: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub choices: Vec<ChatChoice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub struct AzureOpenAIClient {
    config: AzureOpenAIConfig,
    client: Client,
}

impl AzureOpenAIClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();

        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
            .map_err(|_| "AZURE_OPENAI_ENDPOINT not set")?;
        let api_key = std::env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| "AZURE_OPENAI_API_KEY not set")?;
        let api_version = std::env::var("AZURE_OPENAI_API_VERSION").unwrap_or_else(|_| "2023-12-01-preview".to_string());
        let deployment = std::env::var("AZURE_OPENAI_DEPLOYMENT").unwrap_or_else(|_| "gpt-35-turbo".to_string());

        let config = AzureOpenAIConfig {
            endpoint,
            api_key,
            api_version,
            deployment,
        };

        Ok(Self {
            config,
            client: Client::new(),
        })
    }

    pub async fn chat_completions(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<ChatCompletionResponse, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.config.endpoint, self.config.deployment, self.config.api_version
        );

        let request_body = ChatCompletionRequest {
            messages,
            temperature,
            max_tokens,
            top_p: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
        };

        info!("Sending request to Azure OpenAI: {}", url);

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Azure OpenAI API error: {}", error_text);
            return Err(format!("Azure OpenAI API error: {}", error_text).into());
        }

        let completion_response: ChatCompletionResponse = response.json().await?;
        Ok(completion_response)
    }

    pub async fn simple_chat(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ];

        let response = self.chat_completions(messages, 0.7, Some(1000)).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err("No response from AI".into())
        }
    }
}
