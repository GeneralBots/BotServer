use async_trait::async_trait;
use futures::StreamExt;
use langchain_rust::{
    language_models::llm::LLM,
    llm::{claude::Claude, openai::OpenAI},
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::tools::ToolManager;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate(
        &self,
        prompt: &str,
        config: &Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    async fn generate_stream(
        &self,
        prompt: &str,
        config: &Value,
        tx: mpsc::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn generate_with_tools(
        &self,
        prompt: &str,
        config: &Value,
        available_tools: &[String],
        tool_manager: Arc<ToolManager>,
        session_id: &str,
        user_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

pub struct OpenAIClient {
    client: OpenAI<langchain_rust::llm::openai::OpenAIConfig>,
}

impl OpenAIClient {
    pub fn new(client: OpenAI<langchain_rust::llm::openai::OpenAIConfig>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LLMProvider for OpenAIClient {
    async fn generate(
        &self,
        prompt: &str,
        _config: &Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let result = self
            .client
            .invoke(prompt)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(result)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _config: &Value,
        tx: mpsc::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let messages = vec![langchain_rust::schemas::Message::new_human_message(prompt)];
        let mut stream = self
            .client
            .stream(&messages)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    let content = chunk.content;
                    if !content.is_empty() {
                        let _ = tx.send(content.to_string()).await;
                    }
                }
                Err(e) => {
                    eprintln!("Stream error: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn generate_with_tools(
        &self,
        prompt: &str,
        _config: &Value,
        available_tools: &[String],
        _tool_manager: Arc<ToolManager>,
        _session_id: &str,
        _user_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let tools_info = if available_tools.is_empty() {
            String::new()
        } else {
            format!("\n\nAvailable tools: {}. You can suggest using these tools if they would help answer the user's question.", available_tools.join(", "))
        };

        let enhanced_prompt = format!("{}{}", prompt, tools_info);

        let result = self
            .client
            .invoke(&enhanced_prompt)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(result)
    }
}

pub struct AnthropicClient {
    client: Claude,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        let client = Claude::default().with_api_key(api_key);
        Self { client }
    }
}

#[async_trait]
impl LLMProvider for AnthropicClient {
    async fn generate(
        &self,
        prompt: &str,
        _config: &Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let result = self
            .client
            .invoke(prompt)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(result)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _config: &Value,
        tx: mpsc::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let messages = vec![langchain_rust::schemas::Message::new_human_message(prompt)];
        let mut stream = self
            .client
            .stream(&messages)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    let content = chunk.content;
                    if !content.is_empty() {
                        let _ = tx.send(content.to_string()).await;
                    }
                }
                Err(e) => {
                    eprintln!("Stream error: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn generate_with_tools(
        &self,
        prompt: &str,
        _config: &Value,
        available_tools: &[String],
        _tool_manager: Arc<ToolManager>,
        _session_id: &str,
        _user_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let tools_info = if available_tools.is_empty() {
            String::new()
        } else {
            format!("\n\nAvailable tools: {}. You can suggest using these tools if they would help answer the user's question.", available_tools.join(", "))
        };

        let enhanced_prompt = format!("{}{}", prompt, tools_info);

        let result = self
            .client
            .invoke(&enhanced_prompt)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(result)
    }
}

pub struct MockLLMProvider;

impl MockLLMProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LLMProvider for MockLLMProvider {
    async fn generate(
        &self,
        prompt: &str,
        _config: &Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(format!("Mock response to: {}", prompt))
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _config: &Value,
        tx: mpsc::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = format!("Mock stream response to: {}", prompt);
        for word in response.split_whitespace() {
            let _ = tx.send(format!("{} ", word)).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        Ok(())
    }

    async fn generate_with_tools(
        &self,
        prompt: &str,
        _config: &Value,
        available_tools: &[String],
        _tool_manager: Arc<ToolManager>,
        _session_id: &str,
        _user_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let tools_list = if available_tools.is_empty() {
            "no tools available".to_string()
        } else {
            available_tools.join(", ")
        };
        Ok(format!(
            "Mock response with tools [{}] to: {}",
            tools_list, prompt
        ))
    }
}
