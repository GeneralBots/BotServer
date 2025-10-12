use async_trait::async_trait;
use futures::StreamExt;
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
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIClient {
    async fn generate(
        &self,
        prompt: &str,
        _config: &Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": "gpt-3.5-turbo",
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": 1000
            }))
            .send()
            .await?;

        let result: Value = response.json().await?;
        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(content)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _config: &Value,
        tx: mpsc::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": "gpt-3.5-turbo",
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": 1000,
                "stream": true
            }))
            .send()
            .await?;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let chunk_str = String::from_utf8_lossy(&chunk);

            for line in chunk_str.lines() {
                if line.starts_with("data: ") && !line.contains("[DONE]") {
                    if let Ok(data) = serde_json::from_str::<Value>(&line[6..]) {
                        if let Some(content) = data["choices"][0]["delta"]["content"].as_str() {
                            buffer.push_str(content);
                            let _ = tx.send(content.to_string()).await;
                        }
                    }
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

        self.generate(&enhanced_prompt, &Value::Null).await
    }
}

pub struct AnthropicClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicClient {
    async fn generate(
        &self,
        prompt: &str,
        _config: &Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let response = self
            .client
            .post(&format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": "claude-3-sonnet-20240229",
                "max_tokens": 1000,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await?;

        let result: Value = response.json().await?;
        let content = result["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(content)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _config: &Value,
        tx: mpsc::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = self
            .client
            .post(&format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": "claude-3-sonnet-20240229",
                "max_tokens": 1000,
                "messages": [{"role": "user", "content": prompt}],
                "stream": true
            }))
            .send()
            .await?;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let chunk_str = String::from_utf8_lossy(&chunk);

            for line in chunk_str.lines() {
                if line.starts_with("data: ") {
                    if let Ok(data) = serde_json::from_str::<Value>(&line[6..]) {
                        if data["type"] == "content_block_delta" {
                            if let Some(text) = data["delta"]["text"].as_str() {
                                buffer.push_str(text);
                                let _ = tx.send(text.to_string()).await;
                            }
                        }
                    }
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

        self.generate(&enhanced_prompt, &Value::Null).await
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
