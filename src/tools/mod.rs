use async_trait::async_trait;
use redis::AsyncCommands;
use rhai::{Engine, Scope};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::{
    channels::ChannelAdapter,
    session::SessionManager,
    shared::BotResponse,
};

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

pub struct RedisToolExecutor {
    redis_client: redis::Client,
    web_adapter: Arc<dyn ChannelAdapter>,
    voice_adapter: Arc<dyn ChannelAdapter>,
    whatsapp_adapter: Arc<dyn ChannelAdapter>,
}

impl RedisToolExecutor {
    pub fn new(
        redis_url: &str,
        web_adapter: Arc<dyn ChannelAdapter>,
        voice_adapter: Arc<dyn ChannelAdapter>,
        whatsapp_adapter: Arc<dyn ChannelAdapter>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            redis_client: client,
            web_adapter,
            voice_adapter,
            whatsapp_adapter,
        })
    }

    async fn send_tool_message(
        &self,
        session_id: &str,
        user_id: &str,
        channel: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = BotResponse {
            bot_id: "tool_bot".to_string(),
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
            channel: channel.to_string(),
            content: message.to_string(),
            message_type: "tool".to_string(),
            stream_token: None,
            is_complete: true,
        };

        match channel {
            "web" => self.web_adapter.send_message(response).await,
            "voice" => self.voice_adapter.send_message(response).await,
            "whatsapp" => self.whatsapp_adapter.send_message(response).await,
            _ => Ok(()),
        }
    }

    fn create_rhai_engine(&self, session_id: String, user_id: String, channel: String) -> Engine {
        let mut engine = Engine::new();

        let tool_executor = Arc::new((
            self.redis_client.clone(),
            self.web_adapter.clone(),
            self.voice_adapter.clone(),
            self.whatsapp_adapter.clone(),
        ));

        let session_id_clone = session_id.clone();
        let user_id_clone = user_id.clone();
        let channel_clone = channel.clone();

        engine.register_fn("talk", move |message: String| {
            let tool_executor = Arc::clone(&tool_executor);
            let session_id = session_id_clone.clone();
            let user_id = user_id_clone.clone();
            let channel = channel_clone.clone();

            tokio::spawn(async move {
                let (redis_client, web_adapter, voice_adapter, whatsapp_adapter) = &*tool_executor;

                let response = BotResponse {
                    bot_id: "tool_bot".to_string(),
                    user_id: user_id.clone(),
                    session_id: session_id.clone(),
                    channel: channel.clone(),
                    content: message.clone(),
                    message_type: "tool".to_string(),
                    stream_token: None,
                    is_complete: true,
                };

                let result = match channel.as_str() {
                    "web" => web_adapter.send_message(response).await,
                    "voice" => voice_adapter.send_message(response).await,
                    "whatsapp" => whatsapp_adapter.send_message(response).await,
                    _ => Ok(()),
                };

                if let Err(e) = result {
                    log::error!("Failed to send tool message: {}", e);
                }

                if let Ok(mut conn) = redis_client.get_async_connection().await {
                    let output_key = format!("tool:{}:output", session_id);
                    let _ = conn.lpush(&output_key, &message).await;
                }
            });
        });

        let hear_executor = self.redis_client.clone();
        let session_id_clone = session_id.clone();

        engine.register_fn("hear", move || -> String {
            let hear_executor = hear_executor.clone();
            let session_id = session_id_clone.clone();

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                match hear_executor.get_async_connection().await {
                    Ok(mut conn) => {
                        let input_key = format!("tool:{}:input", session_id);
                        let waiting_key = format!("tool:{}:waiting", session_id);

                        let _ = conn.set_ex(&waiting_key, "true", 300).await;
                        let result: Option<(String, String)> =
                            conn.brpop(&input_key, 30).await.ok().flatten();
                        let _ = conn.del(&waiting_key).await;

                        result
                            .map(|(_, input)| input)
                            .unwrap_or_else(|| "timeout".to_string())
                    }
                    Err(e) => {
                        log::error!("HEAR Redis error: {}", e);
                        "error".to_string()
                    }
                }
            })
        });

        engine
    }

    async fn cleanup_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        let keys = vec![
            format!("tool:{}:output", session_id),
            format!("tool:{}:input", session_id),
            format!("tool:{}:waiting", session_id),
            format!("tool:{}:active", session_id),
        ];

        for key in keys {
            let _: () = conn.del(&key).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl ToolExecutor for RedisToolExecutor {
    async fn execute(
        &self,
        tool_name: &str,
        session_id: &str,
        user_id: &str,
    ) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let tool = get_tool(tool_name).ok_or_else(|| format!("Tool not found: {}", tool_name))?;

        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let session_key = format!("tool:{}:session", session_id);
        let session_data = serde_json::json!({
            "user_id": user_id,
            "tool_name": tool_name,
            "started_at": chrono::Utc::now().to_rfc3339(),
        });
        conn.set_ex(&session_key, session_data.to_string(), 3600)
            .await?;

        let active_key = format!("tool:{}:active", session_id);
        conn.set_ex(&active_key, "true", 3600).await?;

        let channel = "web";
        let _engine = self.create_rhai_engine(
            session_id.to_string(),
            user_id.to_string(),
            channel.to_string(),
        );

        let redis_clone = self.redis_client.clone();
        let web_adapter_clone = self.web_adapter.clone();
        let voice_adapter_clone = self.voice_adapter.clone();
        let whatsapp_adapter_clone = self.whatsapp_adapter.clone();
        let session_id_clone = session_id.to_string();
        let user_id_clone = user_id.to_string();
        let tool_script = tool.script.clone();

        tokio::spawn(async move {
            let mut engine = Engine::new();
            let mut scope = Scope::new();

            let redis_client = redis_clone.clone();
            let web_adapter = web_adapter_clone.clone();
            let voice_adapter = voice_adapter_clone.clone();
            let whatsapp_adapter = whatsapp_adapter_clone.clone();
            let session_id = session_id_clone.clone();
            let user_id = user_id_clone.clone();

            engine.register_fn("talk", move |message: String| {
                let redis_client = redis_client.clone();
                let web_adapter = web_adapter.clone();
                let voice_adapter = voice_adapter.clone();
                let whatsapp_adapter = whatsapp_adapter.clone();
                let session_id = session_id.clone();
                let user_id = user_id.clone();

                tokio::spawn(async move {
                    let channel = "web";

                    let response = BotResponse {
                        bot_id: "tool_bot".to_string(),
                        user_id: user_id.clone(),
                        session_id: session_id.clone(),
                        channel: channel.to_string(),
                        content: message.clone(),
                        message_type: "tool".to_string(),
                        stream_token: None,
                        is_complete: true,
                    };

                    let send_result = match channel {
                        "web" => web_adapter.send_message(response).await,
                        "voice" => voice_adapter.send_message(response).await,
                        "whatsapp" => whatsapp_adapter.send_message(response).await,
                        _ => Ok(()),
                    };

                    if let Err(e) = send_result {
                        log::error!("Failed to send tool message: {}", e);
                    }

                    if let Ok(mut conn) = redis_client.get_async_connection().await {
                        let output_key = format!("tool:{}:output", session_id);
                        let _ = conn.lpush(&output_key, &message).await;
                    }
                });
            });

            let hear_redis = redis_clone.clone();
            let session_id_hear = session_id.clone();
            engine.register_fn("hear", move || -> String {
                let hear_redis = hear_redis.clone();
                let session_id = session_id_hear.clone();

                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    match hear_redis.get_async_connection().await {
                        Ok(mut conn) => {
                            let input_key = format!("tool:{}:input", session_id);
                            let waiting_key = format!("tool:{}:waiting", session_id);

                            let _ = conn.set_ex(&waiting_key, "true", 300).await;
                            let result: Option<(String, String)> =
                                conn.brpop(&input_key, 30).await.ok().flatten();
                            let _ = conn.del(&waiting_key).await;

                            result
                                .map(|(_, input)| input)
                                .unwrap_or_else(|| "timeout".to_string())
                        }
                        Err(_) => "error".to_string(),
                    }
                })
            });

            match engine.eval_with_scope::<()>(&mut scope, &tool_script) {
                Ok(_) => {
                    log::info!(
                        "Tool {} completed successfully for session {}",
                        tool_name,
                        session_id
                    );

                    let completion_msg =
                        "🛠️ Tool execution completed. How can I help you with anything else?";
                    let response = BotResponse {
                        bot_id: "tool_bot".to_string(),
                        user_id: user_id_clone,
                        session_id: session_id_clone.clone(),
                        channel: "web".to_string(),
                        content: completion_msg.to_string(),
                        message_type: "tool_complete".to_string(),
                        stream_token: None,
                        is_complete: true,
                    };

                    let _ = web_adapter_clone.send_message(response).await;
                }
                Err(e) => {
                    log::error!("Tool execution failed: {}", e);

                    let error_msg = format!("❌ Tool error: {}", e);
                    let response = BotResponse {
                        bot_id: "tool_bot".to_string(),
                        user_id: user_id_clone,
                        session_id: session_id_clone.clone(),
                        channel: "web".to_string(),
                        content: error_msg,
                        message_type: "tool_error".to_string(),
                        stream_token: None,
                        is_complete: true,
                    };

                    let _ = web_adapter_clone.send_message(response).await;
                }
            }

            if let Ok(mut conn) = redis_clone.get_async_connection().await {
                let active_key = format!("tool:{}:active", session_id_clone);
                let _ = conn.del(&active_key).await;
            }
        });

        Ok(ToolResult {
            success: true,
            output: format!(
                "🛠️ Starting {} tool. Please follow the tool's instructions.",
                tool_name
            ),
            requires_input: true,
            session_id: session_id.to_string(),
        })
    }

    async fn provide_input(
        &self,
        session_id: &str,
        input: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let input_key = format!("tool:{}:input", session_id);
        conn.lpush(&input_key, input).await?;
        Ok(())
    }

    async fn get_output(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let output_key = format!("tool:{}:output", session_id);
        let messages: Vec<String> = conn.lrange(&output_key, 0, -1).await?;
        let _: () = conn.del(&output_key).await?;
        Ok(messages)
    }

    async fn is_waiting_for_input(
        &self,
        session_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let waiting_key = format!("tool:{}:waiting", session_id);
        let exists: bool = conn.exists(&waiting_key).await?;
        Ok(exists)
    }
}

fn get_tool(name: &str) -> Option<Tool> {
    match name {
        "calculator" => Some(Tool {
            name: "calculator".to_string(),
            description: "Perform mathematical calculations".to_string(),
            parameters: HashMap::from([
                ("operation".to_string(), "add|subtract|multiply|divide".to_string()),
                ("a".to_string(), "number".to_string()),
                ("b".to_string(), "number".to_string()),
            ]),
            script: r#"
                let TALK = |message| {
                    talk(message);
                };

                let HEAR = || {
                    hear()
                };

                TALK("🔢 Calculator started!");
                TALK("Please enter the first number:");
                let a = HEAR();
                TALK("Please enter the second number:");
                let b = HEAR();
                TALK("Choose operation: add, subtract, multiply, or divide:");
                let op = HEAR();

                let num_a = a.to_float();
                let num_b = b.to_float();

                if op == "add" {
                    let result = num_a + num_b;
                    TALK("✅ Result: " + a + " + " + b + " = " + result);
                } else if op == "subtract" {
                    let result = num_a - num_b;
                    TALK("✅ Result: " + a + " - " + b + " = " + result);
                } else if op == "multiply" {
                    let result = num_a * num_b;
                    TALK("✅ Result: " + a + " × " + b + " = " + result);
                } else if op == "divide" {
                    if num_b != 0.0 {
                        let result = num_a / num_b;
                        TALK("✅ Result: " + a + " ÷ " + b + " = " + result);
                    } else {
                        TALK("❌ Error: Cannot divide by zero!");
                    }
                } else {
                    TALK("❌ Error: Invalid operation. Please use: add, subtract, multiply, or divide");
                }

                TALK("Calculator session completed. Thank you!");
            "#.to_string(),
        }),
        _ => None,
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
                TALK("Calculator started. Enter first number:");
                let a = HEAR();
                TALK("Enter second number:");
                let b = HEAR();
                TALK("Operation (add/subtract/multiply/divide):");
                let op = HEAR();

                let num_a = a.parse::<f64>().unwrap();
                let num_b = b.parse::<f64>().unwrap();
                let result = if op == "add" {
                    num_a + num_b
                } else if op == "subtract" {
                    num_a - num_b
                } else if op == "multiply" {
                    num_a * num_b
                } else if op == "divide" {
                    if num_b == 0.0 {
                        TALK("Cannot divide by zero");
                        return;
                    }
                    num_a / num_b
                } else {
                    TALK("Invalid operation");
                    return;
                };
                TALK("Result: ".to_string() + &result.to_string());
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
        let tool = self.get_tool(tool_name).ok_or("Tool not found")?;

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
        session_id: &str,
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
        let script = tool.script.clone();
        let session_manager_clone = session_manager.clone();
        let waiting_responses = self.waiting_responses.clone();

        tokio::spawn(async move {
            let mut engine = rhai::Engine::new();
            let (talk_tx, mut talk_rx) = mpsc::channel(100);
            let (hear_tx, mut hear_rx) = mpsc::channel(100);

            {
                let key = format!("{}:{}", user_id, bot_id);
                let mut waiting = waiting_responses.lock().await;
                waiting.insert(key, hear_tx);
            }

            let channel_sender_clone = channel_sender.clone();
            let user_id_clone = user_id.clone();
            let bot_id_clone = bot_id.clone();

            let talk_tx_clone = talk_tx.clone();
            engine.register_fn("TALK", move |message: String| {
                let tx = talk_tx_clone.clone();
                tokio::spawn(async move {
                    let _ = tx.send(message).await;
                });
            });

            let hear_rx_mutex = Arc::new(Mutex::new(hear_rx));
            engine.register_fn("HEAR", move || {
                let hear_rx = hear_rx_mutex.clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        let mut receiver = hear_rx.lock().await;
                        receiver.recv().await.unwrap_or_default()
                    })
                })
            });

            let script_result =
                tokio::task::spawn_blocking(move || engine.eval::<()>(&script)).await;

            if let Ok(Err(e)) = script_result {
                let error_response = BotResponse {
                    bot_id: bot_id_clone.clone(),
                    user_id: user_id_clone.clone(),
                    session_id: Uuid::new_v4().to_string(),
                    channel: "test".to_string(),
                    content: format!("Tool error: {}", e),
                    message_type: "text".to_string(),
                    stream_token: None,
                    is_complete: true,
                };
                let _ = channel_sender_clone.send(error_response).await;
            }

            while let Some(message) = talk_rx.recv().await {
                let response = BotResponse {
                    bot_id: bot_id.clone(),
                    user_id: user_id.clone(),
                    session_id: Uuid::new_v4().to_string(),
                    channel: "test".to_string(),
                    content: message,
                    message_type: "text".to_string(),
                    stream_token: None,
                    is_complete: true,
                };
                let _ = channel_sender.send(response).await;
            }

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
