//! LLM Pipeline - Complete message flow with KB injection and prompt management
//! 
//! This module orchestrates the full pipeline:
//! 1. Load system prompt from configuration
//! 2. Inject KB context (from Qdrant/USE KB)
//! 3. Load conversation history
//! 4. Build messages array with proper token limits
//! 5. Call LLM provider
//! 6. Stream response back

use crate::{openai_client::OpenAIClient, traits::LLMProvider};
use log::{info, trace};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub system_prompt: String,
    pub kb_context: String,
    pub max_tokens: usize,
    pub model: String,
    pub temperature: f32,
    pub top_p: f32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            kb_context: String::new(),
            max_tokens: 16384,
            model: "gpt-3.5-turbo".to_string(),
            temperature: 1.0,
            top_p: 1.0,
        }
    }
}

/// Message builder - constructs the messages array for LLM
pub struct MessageBuilder;

impl MessageBuilder {
    /// Build complete messages array with system prompt, KB context, and history
    /// 
    /// Order:
    /// 1. System prompt (if present)
    /// 2. KB context data (if present) - injected as system message
    /// 3. Conversation history
    /// 4. Current user message
    pub fn build(
        system_prompt: &str,
        kb_context: &str,
        history: &[(String, String)],
        user_message: &str,
    ) -> Value {
        let mut messages = Vec::new();

        // 1. System prompt
        if !system_prompt.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": OpenAIClient::sanitize_utf8(system_prompt)
            }));
        }

        // 2. KB context (injected knowledge base)
        if !kb_context.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": OpenAIClient::sanitize_utf8(kb_context),
                "name": "knowledge_base"
            }));
        }

        // 3. Conversation history
        for (role, content) in history {
            let api_role = match role.as_str() {
                "user" | "assistant" | "system" | "developer" | "tool" => role,
                "episodic" | "compact" => "system",
                _ => "system",
            };
            messages.push(serde_json::json!({
                "role": api_role,
                "content": OpenAIClient::sanitize_utf8(content)
            }));
        }

        // 4. Current user message
        if !user_message.is_empty() {
            messages.push(serde_json::json!({
                "role": "user",
                "content": OpenAIClient::sanitize_utf8(user_message)
            }));
        }

        serde_json::Value::Array(messages)
    }

    /// Calculate estimated tokens for messages
    pub fn estimate_tokens(messages: &Value) -> usize {
        OpenAIClient::estimate_messages_tokens(messages)
    }

    /// Ensure messages don't exceed token limit
    pub fn ensure_token_limit(messages: &Value, model_context_limit: usize) -> Value {
        OpenAIClient::ensure_token_limit(messages, model_context_limit)
    }
}

/// LLM Pipeline executor
pub struct LlmPipeline {
    provider: Arc<dyn LLMProvider>,
    config: PipelineConfig,
}

impl LlmPipeline {
    pub fn new(provider: Arc<dyn LLMProvider>, config: PipelineConfig) -> Self {
        Self { provider, config }
    }

    /// Generate response with full pipeline
    pub async fn generate(
        &self,
        history: &[(String, String)],
        user_message: &str,
        api_key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let messages = MessageBuilder::build(
            &self.config.system_prompt,
            &self.config.kb_context,
            history,
            user_message,
        );

        let messages = MessageBuilder::ensure_token_limit(
            &messages,
            self.get_context_limit(&self.config.model)
        );

        trace!("Pipeline: Calling LLM with {} messages", 
            messages.as_array().map(|a| a.len()).unwrap_or(0));

        self.provider.generate(
            user_message,
            &messages,
            &self.config.model,
            api_key,
        ).await
    }

    /// Stream response with full pipeline
    pub async fn generate_stream(
        &self,
        history: &[(String, String)],
        user_message: &str,
        api_key: &str,
        tx: mpsc::Sender<String>,
        tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let messages = MessageBuilder::build(
            &self.config.system_prompt,
            &self.config.kb_context,
            history,
            user_message,
        );

        let messages = MessageBuilder::ensure_token_limit(
            &messages,
            self.get_context_limit(&self.config.model)
        );

        trace!("Pipeline: Streaming LLM with {} messages", 
            messages.as_array().map(|a| a.len()).unwrap_or(0));

        self.provider.generate_stream(
            user_message,
            &messages,
            tx,
            &self.config.model,
            api_key,
            tools,
        ).await
    }

    fn get_context_limit(&self, model: &str) -> usize {
        if model.contains("glm-4") || model.contains("GLM-4") {
            202750
        } else if model.contains("gemini") {
            1000000
        } else if model.contains("gpt-oss") || model.contains("gpt-4") {
            128000
        } else if model.contains("gpt-3.5") {
            16385
        } else if model.starts_with("http://localhost:808") || model == "local" {
            768
        } else {
            32768
        }
    }
}

/// KB Context Manager - handles knowledge base injection
pub struct KbContextManager;

impl KbContextManager {
    /// Search and inject KB context based on user query
    /// Returns formatted KB context string for injection
    pub async fn inject_kb(
        _bot_id: &str,
        bot_name: &str,
        query: &str,
        _max_chunks: usize,
        _max_len: usize,
    ) -> String {
        // This would call the KB service (Qdrant) to retrieve relevant chunks
        // For now, returns empty - KB retrieval lives in the `sources`/Qdrant path
        info!("KB injection requested: bot={}, query={}", bot_name, query);
        
        // Placeholder - actual KB retrieval would go here
        // The real implementation would:
        // 1. Embed the query
        // 2. Search Qdrant for similar chunks
        // 3. Format chunks into context string
        // 4. Return formatted context
        
        String::new()
    }

    /// Clear KB context for a session
    pub fn clear_kb(session_id: &str) {
        info!("KB context cleared for session: {}", session_id);
    }
}

/// Prompt Manager - handles system prompt templates
pub struct PromptManager;

impl PromptManager {
    /// Get system prompt template by name
    pub fn get_template(name: &str) -> String {
        // Load from prompts.csv or default templates
        match name {
            "default" => "You are a helpful assistant.".to_string(),
            "customer_service" => "You are a customer service representative. Be polite and helpful.".to_string(),
            "sales" => "You are a sales assistant. Help the customer find the right product.".to_string(),
            _ => format!("You are a {} assistant.", name)
        }
    }

    /// Format prompt with variables
    pub fn format_prompt(template: &str, vars: &std::collections::HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_messages() {
        let messages = MessageBuilder::build(
            "You are helpful",
            "KB context here",
            &[("user".to_string(), "Hello".to_string())],
            "How are you?"
        );

        let arr = messages.as_array().unwrap();
        assert_eq!(arr.len(), 4); // system + kb + history + user
        assert_eq!(arr[0]["role"], "system");
        assert_eq!(arr[1]["role"], "system");
        assert_eq!(arr[2]["role"], "user");
        assert_eq!(arr[3]["role"], "user");
    }

    #[test]
    fn test_empty_kb() {
        let messages = MessageBuilder::build(
            "System prompt",
            "", // empty KB
            &[],
            "Test"
        );

        let arr = messages.as_array().unwrap();
        assert_eq!(arr.len(), 2); // system + user only
    }
}
