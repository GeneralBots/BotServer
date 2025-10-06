use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::{session::SessionManager, shared::BotResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub requires_input: bool,
    pub session_id: String,
}

#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, String>,
    pub script: String,
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(
        &self,
        tool_name: &str,
        session_id: &str,
        user_id: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>>;
    async fn provide_input(
        &self,
        session_id: &str,
        input: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn get_output(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn is_waiting_for_input(
        &self,
        session_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}

pub struct MockToolExecutor;

impl MockToolExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolExecutor for MockToolExecutor {
    async fn execute(
        &self,
        tool_name: &str,
        session_id: &str,
        user_id: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(ToolResult {
            success: true,
            output: format!("Mock tool {} executed for user {}", tool_name, user_id),
            requires_input: false,
            session_id: session_id.to_string(),
        })
    }

    async fn provide_input(
        &self,
        _session_id: &str,
        _input: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn get_output(
        &self,
        _session_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec!["Mock output".to_string()])
    }

    async fn is_waiting_for_input(
        &self,
        _session_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(false)
    }
}

#[derive(Clone)]
pub struct ToolManager {
    tools: HashMap<String, Tool>,
    waiting_responses: Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>,
}

impl ToolManager {
    pub fn new() -> Self {
        let mut tools = HashMap::new();

        let calculator_tool = Tool {
            name: "calculator".to_string(),
            description: "Perform calculations".to_string(),
            parameters: HashMap::from([
                (
                    "operation".to_string(),
                    "add|subtract|multiply|divide".to_string(),
                ),
                ("a".to_string(), "number".to_string()),
                ("b".to_string(), "number".to_string()),
            ]),
            script: r#"
                // Calculator tool implementation
                print("Calculator started");
            "#
            .to_string(),
        };

        tools.insert(calculator_tool.name.clone(), calculator_tool);
        Self {
            tools,
            waiting_responses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub async fn execute_tool(
        &self,
        tool_name: &str,
        session_id: &str,
        user_id: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let _tool = self.get_tool(tool_name).ok_or("Tool not found")?;

        Ok(ToolResult {
            success: true,
            output: format!("Tool {} started for user {}", tool_name, user_id),
            requires_input: true,
            session_id: session_id.to_string(),
        })
    }

    pub async fn is_tool_waiting(
        &self,
        session_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let waiting = self.waiting_responses.lock().await;
        Ok(waiting.contains_key(session_id))
    }

    pub async fn provide_input(
        &self,
        session_id: &str,
        input: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.provide_user_response(session_id, "default_bot", input.to_string())
            .await
    }

    pub async fn get_tool_output(
        &self,
        _session_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![])
    }

    pub async fn execute_tool_with_session(
        &self,
        tool_name: &str,
        user_id: &str,
        bot_id: &str,
        session_manager: SessionManager,
        channel_sender: mpsc::Sender<BotResponse>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tool = self.get_tool(tool_name).ok_or("Tool not found")?;
        session_manager
            .set_current_tool(user_id, bot_id, Some(tool_name.to_string()))
            .await?;

        let user_id = user_id.to_string();
        let bot_id = bot_id.to_string();
        let _script = tool.script.clone();
        let session_manager_clone = session_manager.clone();
        let waiting_responses = self.waiting_responses.clone();

        tokio::spawn(async move {
            // Simulate tool execution
            let response = BotResponse {
                bot_id: bot_id.clone(),
                user_id: user_id.clone(),
                session_id: Uuid::new_v4().to_string(),
                channel: "test".to_string(),
                content: format!("Tool {} executed successfully", tool_name),
                message_type: "text".to_string(),
                stream_token: None,
                is_complete: true,
            };
            let _ = channel_sender.send(response).await;

            let _ = session_manager_clone
                .set_current_tool(&user_id, &bot_id, None)
                .await;
        });

        Ok(())
    }

    pub async fn provide_user_response(
        &self,
        user_id: &str,
        bot_id: &str,
        response: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("{}:{}", user_id, bot_id);
        let mut waiting = self.waiting_responses.lock().await;
        if let Some(tx) = waiting.get_mut(&key) {
            let _ = tx.send(response).await;
            waiting.remove(&key);
        }
        Ok(())
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ToolApi;

impl ToolApi {
    pub fn new() -> Self {
        Self
    }
}
