use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_ws::Message as WsMessage;
use chrono::Utc;
use log::{debug, error, info, trace, warn};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::channels::ChannelAdapter;
use crate::shared::models::{BotResponse, UserMessage, UserSession};
use crate::shared::state::AppState;

pub struct BotOrchestrator {
    pub state: Arc<AppState>,
}

impl BotOrchestrator {
    pub fn new(state: Arc<AppState>) -> Self {
        info!("Creating new BotOrchestrator instance");
        Self { state }
    }

    pub async fn handle_user_input(
        &self,
        session_id: Uuid,
        user_input: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "Handling user input for session {}: '{}'",
            session_id, user_input
        );
        let mut session_manager = self.state.session_manager.lock().await;
        session_manager.provide_input(session_id, user_input.to_string())?;
        debug!("User input handled for session {}", session_id);
        Ok(None)
    }

    pub async fn is_waiting_for_input(&self, session_id: Uuid) -> bool {
        trace!("Checking if session {} is waiting for input", session_id);
        let session_manager = self.state.session_manager.lock().await;
        let result = session_manager.is_waiting_for_input(&session_id);
        trace!("Session {} waiting for input: {}", session_id, result);
        result
    }

    pub fn add_channel(&self, channel_type: &str, adapter: Arc<dyn ChannelAdapter>) {
        info!("Adding channel adapter for type: {}", channel_type);
        self.state
            .channels
            .lock()
            .unwrap()
            .insert(channel_type.to_string(), adapter);
        debug!("Channel adapter for {} added successfully", channel_type);
    }

    pub async fn register_response_channel(
        &self,
        session_id: String,
        sender: mpsc::Sender<BotResponse>,
    ) {
        debug!("Registering response channel for session: {}", session_id);
        self.state
            .response_channels
            .lock()
            .await
            .insert(session_id.clone(), sender);
        trace!("Response channel registered for session: {}", session_id);
    }

    pub async fn set_user_answer_mode(
        &self,
        user_id: &str,
        bot_id: &str,
        mode: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Setting answer mode for user {} with bot {} to mode {}",
            user_id, bot_id, mode
        );
        let mut session_manager = self.state.session_manager.lock().await;
        session_manager.update_answer_mode(user_id, bot_id, mode)?;
        debug!("Answer mode updated successfully");
        Ok(())
    }

    pub async fn send_event(
        &self,
        user_id: &str,
        bot_id: &str,
        session_id: &str,
        channel: &str,
        event_type: &str,
        data: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "Sending event '{}' to session {} on channel {}",
            event_type, session_id, channel
        );

        let event_response = BotResponse {
            bot_id: bot_id.to_string(),
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
            channel: channel.to_string(),
            content: serde_json::to_string(&serde_json::json!({
                "event": event_type,
                "data": data
            }))?,
            message_type: 2,
            stream_token: None,
            is_complete: true,
        };

        trace!("Event response created: {:?}", event_response);

        if let Some(adapter) = self.state.channels.lock().unwrap().get(channel) {
            adapter.send_message(event_response).await?;
            debug!("Event sent successfully via channel adapter");
        } else {
            warn!("No channel adapter found for channel: {}", channel);
        }
        Ok(())
    }

    pub async fn process_message(
        &self,
        message: UserMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Processing message from channel: {}, user: {}, session: {}",
            message.channel, message.user_id, message.session_id
        );
        debug!(
            "Message content: '{}', type: {}",
            message.content, message.message_type
        );

        let user_id = Uuid::parse_str(&message.user_id).unwrap_or_else(|_| {
            let new_id = Uuid::new_v4();
            warn!("Invalid user ID provided, generated new UUID: {}", new_id);
            new_id
        });
        let bot_id = Uuid::parse_str(&message.bot_id)
            .unwrap_or_else(|_| Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());

        debug!("Parsed user_id: {}, bot_id: {}", user_id, bot_id);

        let session = {
            let mut session_manager = self.state.session_manager.lock().await;
            match session_manager.get_user_session(user_id, bot_id)? {
                Some(session) => {
                    debug!("Found existing session: {}", session.id);
                    session
                }
                None => {
                    info!(
                        "Creating new session for user {} with bot {}",
                        user_id, bot_id
                    );
                    let new_session =
                        session_manager.create_session(user_id, bot_id, "New Conversation")?;
                    debug!("New session created: {}", new_session.id);
                    Self::run_start_script(&new_session, Arc::clone(&self.state)).await;
                    new_session
                }
            }
        };

        trace!("Current session state: {:?}", session);

        if self.is_waiting_for_input(session.id).await {
            debug!(
                "Session {} is waiting for input, processing as variable input",
                session.id
            );
            if let Some(variable_name) =
                self.handle_user_input(session.id, &message.content).await?
            {
                info!(
                    "Stored user input in variable '{}' for session {}",
                    variable_name, session.id
                );

                if let Some(adapter) = self.state.channels.lock().unwrap().get(&message.channel) {
                    let ack_response = BotResponse {
                        bot_id: message.bot_id.clone(),
                        user_id: message.user_id.clone(),
                        session_id: message.session_id.clone(),
                        channel: message.channel.clone(),
                        content: format!("Input stored in '{}'", variable_name),
                        message_type: 1,
                        stream_token: None,
                        is_complete: true,
                    };
                    adapter.send_message(ack_response).await?;
                    debug!("Acknowledgment sent for variable storage");
                }
                return Ok(());
            }
        }

        if session.answer_mode == 1 && session.current_tool.is_some() {
            debug!("Session in answer mode with active tool, providing user response");
            self.state.tool_manager.provide_user_response(
                &message.user_id,
                &message.bot_id,
                message.content.clone(),
            )?;
            trace!("User response provided to tool manager");
            return Ok(());
        }

        {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.save_message(
                session.id,
                user_id,
                1,
                &message.content,
                message.message_type,
            )?;
            debug!("User message saved to session history");
        }

        let response_content = self.direct_mode_handler(&message, &session).await?;
        debug!("Generated response content: '{}'", response_content);

        {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.save_message(session.id, user_id, 2, &response_content, 1)?;
            debug!("Bot response saved to session history");
        }

        let bot_response = BotResponse {
            bot_id: message.bot_id,
            user_id: message.user_id,
            session_id: message.session_id.clone(),
            channel: message.channel.clone(),
            content: response_content,
            message_type: 1,
            stream_token: None,
            is_complete: true,
        };

        trace!("Final bot response: {:?}", bot_response);

        if let Some(adapter) = self.state.channels.lock().unwrap().get(&message.channel) {
            adapter.send_message(bot_response).await?;
            info!("Response sent successfully via channel adapter");
        } else {
            warn!("No channel adapter found for channel: {}", message.channel);
        }

        Ok(())
    }

    async fn direct_mode_handler(
        &self,
        message: &UserMessage,
        session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Using direct mode handler for session {}", session.id);

        let history = {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.get_conversation_history(session.id, session.user_id)?
        };

        debug!("Retrieved {} history entries", history.len());

        let mut prompt = String::new();
        for (role, content) in history {
            prompt.push_str(&format!("{}: {}\n", role, content));
        }
        prompt.push_str(&format!("User: {}\nAssistant:", message.content));

        trace!("Constructed prompt for LLM: {}", prompt);

        self.state
            .llm_provider
            .generate(&prompt, &serde_json::Value::Null)
            .await
    }

    pub async fn stream_response(
        &self,
        message: UserMessage,
        response_tx: mpsc::Sender<BotResponse>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Streaming response for user: {}, session: {}",
            message.user_id, message.session_id
        );
        debug!("Message content: '{}'", message.content);

        let mut user_id = Uuid::parse_str(&message.user_id).unwrap_or_else(|_| {
            let new_id = Uuid::new_v4();
            warn!("Invalid user ID, generated new: {}", new_id);
            new_id
        });
        let bot_id = Uuid::parse_str(&message.bot_id).unwrap_or_else(|_| {
            warn!("Invalid bot ID, using nil UUID");
            Uuid::nil()
        });

        debug!("User ID: {}, Bot ID: {}", user_id, bot_id);

        let mut auth = self.state.auth_service.lock().await;
        let user_exists = auth.get_user_by_id(user_id)?;

        if user_exists.is_none() {
            debug!("User {} not found, creating anonymous user", user_id);
            user_id = auth.create_user("anonymous1", "anonymous@local", "password")?;
            info!("Created new anonymous user: {}", user_id);
        } else {
            user_id = user_exists.unwrap().id;
            debug!("Found existing user: {}", user_id);
        }

        let session = {
            let mut sm = self.state.session_manager.lock().await;
            match sm.get_user_session(user_id, bot_id)? {
                Some(sess) => {
                    debug!("Using existing session: {}", sess.id);
                    sess
                }
                None => {
                    info!("Creating new session for streaming");
                    let new_session = sm.create_session(user_id, bot_id, "New Conversation")?;
                    debug!("New session created: {}", new_session.id);
                    Self::run_start_script(&new_session, Arc::clone(&self.state)).await;
                    new_session
                }
            }
        };

        trace!("Session state: {:?}", session);

        if session.answer_mode == 1 && session.current_tool.is_some() {
            debug!("Session in answer mode, forwarding to tool manager");
            self.state.tool_manager.provide_user_response(
                &message.user_id,
                &message.bot_id,
                message.content.clone(),
            )?;
            return Ok(());
        }

        {
            let mut sm = self.state.session_manager.lock().await;
            sm.save_message(
                session.id,
                user_id,
                1,
                &message.content,
                message.message_type,
            )?;
            debug!("User message saved for streaming session");
        }

        let prompt = {
            let mut sm = self.state.session_manager.lock().await;
            let history = sm.get_conversation_history(session.id, user_id)?;
            let mut p = String::new();
            for (role, content) in &history {
                p.push_str(&format!("{}: {}\n", role, content));
            }
            p.push_str(&format!("User: {}\nAssistant:", message.content));
            debug!(
                "Stream prompt constructed with {} history entries",
                history.len()
            );
            trace!("Full prompt: {}", p);
            p
        };

        let (stream_tx, mut stream_rx) = mpsc::channel::<String>(100);
        let llm = self.state.llm_provider.clone();

        if message.channel == "web" {
            debug!("Sending thinking start event for web channel");
            self.send_event(
                &message.user_id,
                &message.bot_id,
                &message.session_id,
                &message.channel,
                "thinking_start",
                serde_json::json!({}),
            )
            .await?;
        } else {
            debug!("Sending thinking message for non-web channel");
            let thinking_response = BotResponse {
                bot_id: message.bot_id.clone(),
                user_id: message.user_id.clone(),
                session_id: message.session_id.clone(),
                channel: message.channel.clone(),
                content: "Thinking...".to_string(),
                message_type: 1,
                stream_token: None,
                is_complete: true,
            };
            response_tx.send(thinking_response).await?;
        }

        info!("Starting LLM stream generation");
        tokio::spawn(async move {
            if let Err(e) = llm
                .generate_stream(&prompt, &serde_json::Value::Null, stream_tx)
                .await
            {
                error!("LLM streaming error: {}", e);
            } else {
                debug!("LLM stream generation completed");
            }
        });

        let mut full_response = String::new();
        let mut analysis_buffer = String::new();
        let mut in_analysis = false;
        let mut chunk_count = 0;

        debug!("Starting to process stream chunks");
        while let Some(chunk) = stream_rx.recv().await {
            chunk_count += 1;
            trace!("Received chunk {}: '{}'", chunk_count, chunk);

            analysis_buffer.push_str(&chunk);

            if analysis_buffer.contains("<|channel|>") {
                debug!("Analysis section started");
                in_analysis = true;
            }

            if in_analysis {
                if analysis_buffer.ends_with("final<|message|>") {
                    debug!(
                        "Analysis section completed, buffer length: {}",
                        analysis_buffer.len()
                    );
                    in_analysis = false;
                    analysis_buffer.clear();

                    if message.channel == "web" {
                        let orchestrator = BotOrchestrator::new(Arc::clone(&self.state));
                        orchestrator
                            .send_event(
                                &message.user_id,
                                &message.bot_id,
                                &message.session_id,
                                &message.channel,
                                "thinking_end",
                                serde_json::json!({
                                    "user_id": message.user_id.clone()
                                }),
                            )
                            .await
                            .ok();
                    }
                    analysis_buffer.clear();
                    continue;
                }

                trace!("Skipping analysis chunk");
                continue;
            }

            full_response.push_str(&chunk);

            let partial = BotResponse {
                bot_id: message.bot_id.clone(),
                user_id: message.user_id.clone(),
                session_id: message.session_id.clone(),
                channel: message.channel.clone(),
                content: chunk,
                message_type: 1,
                stream_token: None,
                is_complete: false,
            };

            if response_tx.send(partial).await.is_err() {
                warn!("Response channel closed, stopping stream processing");
                break;
            }
        }

        debug!(
            "Stream processing completed, {} chunks processed",
            chunk_count
        );
        info!("Full response length: {} characters", full_response.len());

        {
            let mut sm = self.state.session_manager.lock().await;
            sm.save_message(session.id, user_id, 2, &full_response, 1)?;
            debug!("Stream response saved to session history");
        }

        let final_msg = BotResponse {
            bot_id: message.bot_id,
            user_id: message.user_id,
            session_id: message.session_id,
            channel: message.channel,
            content: String::new(),
            message_type: 1,
            stream_token: None,
            is_complete: true,
        };

        response_tx.send(final_msg).await?;
        debug!("Final stream message sent");

        Ok(())
    }

    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Getting sessions for user: {}", user_id);
        let mut session_manager = self.state.session_manager.lock().await;
        let sessions = session_manager.get_user_sessions(user_id)?;
        debug!("Found {} sessions for user {}", sessions.len(), user_id);
        Ok(sessions)
    }

    pub async fn get_conversation_history(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "Getting conversation history for session {} user {}",
            session_id, user_id
        );
        let mut session_manager = self.state.session_manager.lock().await;
        let history = session_manager.get_conversation_history(session_id, user_id)?;
        debug!("Retrieved {} history entries", history.len());
        Ok(history)
    }

    pub async fn process_message_with_tools(
        &self,
        message: UserMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Processing message with tools from user: {}, session: {}",
            message.user_id, message.session_id
        );
        debug!("Message content: '{}'", message.content);

        let user_id = Uuid::parse_str(&message.user_id).unwrap_or_else(|_| {
            let new_id = Uuid::new_v4();
            warn!("Invalid user ID, generated new: {}", new_id);
            new_id
        });
        let bot_id = Uuid::parse_str(&message.bot_id)
            .unwrap_or_else(|_| Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());

        let session = {
            let mut session_manager = self.state.session_manager.lock().await;
            match session_manager.get_user_session(user_id, bot_id)? {
                Some(session) => {
                    debug!("Found existing session: {}", session.id);
                    session
                }
                None => {
                    info!("Creating new session for tools processing");
                    let new_session =
                        session_manager.create_session(user_id, bot_id, "New Conversation")?;
                    debug!("New session created: {}", new_session.id);
                    Self::run_start_script(&new_session, Arc::clone(&self.state)).await;
                    new_session
                }
            }
        };

        {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.save_message(
                session.id,
                user_id,
                1,
                &message.content,
                message.message_type,
            )?;
            debug!("User message saved for tools processing");
        }

        let is_tool_waiting = self
            .state
            .tool_manager
            .is_tool_waiting(&message.session_id)
            .await
            .unwrap_or(false);

        if is_tool_waiting {
            debug!(
                "Tool is waiting for input, providing: '{}'",
                message.content
            );
            self.state
                .tool_manager
                .provide_input(&message.session_id, &message.content)
                .await?;

            if let Ok(tool_output) = self
                .state
                .tool_manager
                .get_tool_output(&message.session_id)
                .await
            {
                debug!("Retrieved {} tool output entries", tool_output.len());
                for output in tool_output {
                    let bot_response = BotResponse {
                        bot_id: message.bot_id.clone(),
                        user_id: message.user_id.clone(),
                        session_id: message.session_id.clone(),
                        channel: message.channel.clone(),
                        content: output.clone(),
                        message_type: 1,
                        stream_token: None,
                        is_complete: true,
                    };

                    if let Some(adapter) = self.state.channels.lock().unwrap().get(&message.channel)
                    {
                        adapter.send_message(bot_response).await?;
                        debug!("Tool output sent: '{}'", output);
                    }
                }
            }
            return Ok(());
        }

        let response = if message.content.to_lowercase().contains("calculator")
            || message.content.to_lowercase().contains("calculate")
            || message.content.to_lowercase().contains("math")
        {
            debug!("Message requires calculator tool");
            match self
                .state
                .tool_manager
                .execute_tool("calculator", &message.session_id, &message.user_id)
                .await
            {
                Ok(tool_result) => {
                    debug!("Calculator tool executed successfully");
                    let mut session_manager = self.state.session_manager.lock().await;
                    session_manager.save_message(session.id, user_id, 2, &tool_result.output, 2)?;
                    tool_result.output
                }
                Err(e) => {
                    error!("Calculator tool error: {}", e);
                    format!("I encountered an error starting the calculator: {}", e)
                }
            }
        } else {
            debug!("Using LLM for response generation");
            let available_tools = self.state.tool_manager.list_tools();
            let tools_context = if !available_tools.is_empty() {
                format!("\n\nAvailable tools: {}. If the user needs calculations, suggest using the calculator tool.", available_tools.join(", "))
            } else {
                String::new()
            };

            let full_prompt = format!("{}{}", message.content, tools_context);
            trace!("Full prompt with tools context: {}", full_prompt);

            self.state
                .llm_provider
                .generate(&full_prompt, &serde_json::Value::Null)
                .await?
        };

        debug!("Generated response: '{}'", response);

        {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.save_message(session.id, user_id, 2, &response, 1)?;
            debug!("Response saved to session history");
        }

        let bot_response = BotResponse {
            bot_id: message.bot_id,
            user_id: message.user_id,
            session_id: message.session_id.clone(),
            channel: message.channel.clone(),
            content: response,
            message_type: 1,
            stream_token: None,
            is_complete: true,
        };

        if let Some(adapter) = self.state.channels.lock().unwrap().get(&message.channel) {
            adapter.send_message(bot_response).await?;
            info!("Tools response sent successfully");
        } else {
            warn!("No channel adapter found for channel: {}", message.channel);
        }

        Ok(())
    }

    async fn run_start_script(session: &UserSession, state: Arc<AppState>) {
        info!("Running start script for session: {}", session.id);
        
        let start_script_path = "start.bas";
        let start_script = match std::fs::read_to_string(start_script_path) {
            Ok(content) => {
                debug!("Loaded start script from {}", start_script_path);
                content
            }
            Err(_) => {
                debug!("No start.bas found, using default welcome script");
                r#"TALK "Welcome to General Bots!""#.to_string()
            }
        };

        debug!("Start script content for session {}: {}", session.id, start_script);

        let session_clone = session.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            let state_for_run = state_clone.clone();
            match crate::basic::ScriptService::new(state_clone, session_clone.clone())
                .compile(&start_script)
                .and_then(|ast| {
                    crate::basic::ScriptService::new(state_for_run, session_clone.clone()).run(&ast)
                }) {
                Ok(_) => {
                    info!(
                        "Start script executed successfully for session {}",
                        session_clone.id
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to run start script for session {}: {}",
                        session_clone.id, e
                    );
                }
            }
        });
    }

    pub async fn send_warning(
        &self,
        session_id: &str,
        channel: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        warn!(
            "Sending warning to session {} on channel {}: {}",
            session_id, channel, message
        );

        if channel == "web" {
            debug!("Sending warning as web event");
            self.send_event(
                "system",
                "system",
                session_id,
                channel,
                "warn",
                serde_json::json!({
                    "message": message,
                    "timestamp": Utc::now().to_rfc3339()
                }),
            )
            .await
        } else {
            debug!("Sending warning as regular message");
            if let Some(adapter) = self.state.channels.lock().unwrap().get(channel) {
                let warn_response = BotResponse {
                    bot_id: "system".to_string(),
                    user_id: "system".to_string(),
                    session_id: session_id.to_string(),
                    channel: channel.to_string(),
                    content: format!("⚠️ WARNING: {}", message),
                    message_type: 1,
                    stream_token: None,
                    is_complete: true,
                };
                adapter.send_message(warn_response).await
            } else {
                warn!(
                    "No channel adapter found for warning on channel: {}",
                    channel
                );
                Ok(())
            }
        }
    }
}

impl Default for BotOrchestrator {
    fn default() -> Self {
        info!("Creating default BotOrchestrator");
        Self {
            state: Arc::new(AppState::default()),
        }
    }
}

#[actix_web::get("/ws")]
async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    info!("WebSocket connection attempt");

    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    let session_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::channel::<BotResponse>(100);

    info!("WebSocket session established: {}", session_id);

    let orchestrator = BotOrchestrator::new(Arc::clone(&data));
    orchestrator
        .register_response_channel(session_id.clone(), tx.clone())
        .await;
    data.web_adapter
        .add_connection(session_id.clone(), tx.clone())
        .await;
    data.voice_adapter
        .add_connection(session_id.clone(), tx.clone())
        .await;

    orchestrator
        .send_event(
            "default_user",
            "default_bot",
            &session_id,
            "web",
            "session_start",
            serde_json::json!({
                "session_id": session_id,
                "timestamp": Utc::now().to_rfc3339()
            }),
        )
        .await
        .ok();
    let web_adapter = data.web_adapter.clone();
    let session_id_clone1 = session_id.clone();
    let session_id_clone2 = session_id.clone();

    actix_web::rt::spawn(async move {
        info!(
            "Starting WebSocket sender for session {}",
            session_id_clone1
        );
        let mut message_count = 0;
        while let Some(msg) = rx.recv().await {
            message_count += 1;
            if let Ok(json) = serde_json::to_string(&msg) {
                trace!("Sending WebSocket message {}: {}", message_count, json);
                if let Err(e) = session.text(json).await {
                    warn!("Failed to send WebSocket message {}: {}", message_count, e);
                    break;
                }
            }
        }
        info!(
            "WebSocket sender terminated for session {}, sent {} messages",
            session_id_clone1, message_count
        );
    });

    actix_web::rt::spawn(async move {
        info!(
            "Starting WebSocket receiver for session {}",
            session_id_clone2
        );
        let mut message_count = 0;
        while let Some(Ok(msg)) = msg_stream.recv().await {
            match msg {
                WsMessage::Text(text) => {
                    message_count += 1;
                    debug!("Received WebSocket message {}: {}", message_count, text);

                    let user_message = UserMessage {
                        bot_id: "default_bot".to_string(),
                        user_id: "default_user".to_string(),
                        session_id: session_id_clone2.clone(),
                        channel: "web".to_string(),
                        content: text.to_string(),
                        message_type: 1,
                        media_url: None,
                        timestamp: Utc::now(),
                    };

                    if let Err(e) = orchestrator.stream_response(user_message, tx.clone()).await {
                        error!(
                            "Error processing WebSocket message {}: {}",
                            message_count, e
                        );
                    }
                }
                WsMessage::Close(_) => {
                    info!("WebSocket close received for session {}", session_id_clone2);
                    orchestrator
                        .send_event(
                            "default_user",
                            "default_bot",
                            &session_id_clone2,
                            "web",
                            "session_end",
                            serde_json::json!({}),
                        )
                        .await
                        .ok();

                    web_adapter.remove_connection(&session_id_clone2).await;
                    break;
                }
                _ => {
                    trace!("Received non-text WebSocket message");
                }
            }
        }
        info!(
            "WebSocket receiver terminated for session {}, processed {} messages",
            session_id_clone2, message_count
        );
    });

    info!(
        "WebSocket handler setup completed for session {}",
        session_id
    );
    Ok(res)
}

#[actix_web::get("/api/whatsapp/webhook")]
async fn whatsapp_webhook_verify(
    data: web::Data<AppState>,
    web::Query(params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    info!("WhatsApp webhook verification request");

    let empty = String::new();
    let mode = params.get("hub.mode").unwrap_or(&empty);
    let token = params.get("hub.verify_token").unwrap_or(&empty);
    let challenge = params.get("hub.challenge").unwrap_or(&empty);

    debug!(
        "Verification params - mode: {}, token: {}, challenge: {}",
        mode, token, challenge
    );

    match data.whatsapp_adapter.verify_webhook(mode, token, challenge) {
        Ok(challenge_response) => {
            info!("WhatsApp webhook verification successful");
            Ok(HttpResponse::Ok().body(challenge_response))
        }
        Err(_) => {
            warn!("WhatsApp webhook verification failed");
            Ok(HttpResponse::Forbidden().body("Verification failed"))
        }
    }
}

#[actix_web::post("/api/whatsapp/webhook")]
async fn whatsapp_webhook(
    data: web::Data<AppState>,
    payload: web::Json<crate::whatsapp::WhatsAppMessage>,
) -> Result<HttpResponse> {
    info!("WhatsApp webhook message received");

    match data
        .whatsapp_adapter
        .process_incoming_message(payload.into_inner())
        .await
    {
        Ok(user_messages) => {
            info!("Processed {} WhatsApp messages", user_messages.len());
            for user_message in user_messages {
                let orchestrator = BotOrchestrator::new(Arc::clone(&data));
                if let Err(e) = orchestrator.process_message(user_message).await {
                    error!("Error processing WhatsApp message: {}", e);
                }
            }
            Ok(HttpResponse::Ok().body(""))
        }
        Err(e) => {
            error!("Error processing WhatsApp webhook: {}", e);
            Ok(HttpResponse::BadRequest().body("Invalid message"))
        }
    }
}

#[actix_web::post("/api/voice/start")]
async fn voice_start(
    data: web::Data<AppState>,
    info: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let session_id = info
        .get("session_id")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let user_id = info
        .get("user_id")
        .and_then(|u| u.as_str())
        .unwrap_or("user");

    info!(
        "Voice session start request - session: {}, user: {}",
        session_id, user_id
    );

    match data
        .voice_adapter
        .start_voice_session(session_id, user_id)
        .await
    {
        Ok(token) => {
            info!(
                "Voice session started successfully for session {}",
                session_id
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({"token": token, "status": "started"})))
        }
        Err(e) => {
            error!(
                "Failed to start voice session for session {}: {}",
                session_id, e
            );
            Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()})))
        }
    }
}

#[actix_web::post("/api/voice/stop")]
async fn voice_stop(
    data: web::Data<AppState>,
    info: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let session_id = info
        .get("session_id")
        .and_then(|s| s.as_str())
        .unwrap_or("");

    info!("Voice session stop request - session: {}", session_id);

    match data.voice_adapter.stop_voice_session(session_id).await {
        Ok(()) => {
            info!(
                "Voice session stopped successfully for session {}",
                session_id
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({"status": "stopped"})))
        }
        Err(e) => {
            error!(
                "Failed to stop voice session for session {}: {}",
                session_id, e
            );
            Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()})))
        }
    }
}

#[actix_web::post("/api/sessions")]
async fn create_session(data: web::Data<AppState>) -> Result<HttpResponse> {
    info!("Creating new session");

    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let bot_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();

    let session = {
        let mut session_manager = data.session_manager.lock().await;
        match session_manager.create_session(user_id, bot_id, "New Conversation") {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create session: {}", e);
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": e.to_string()})));
            }
        }
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "session_id": session.id,
        "title": "New Conversation",
        "created_at": Utc::now()
    })))
}

#[actix_web::get("/api/sessions")]
async fn get_sessions(data: web::Data<AppState>) -> Result<HttpResponse> {
    info!("Getting sessions list");
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let orchestrator = BotOrchestrator::new(Arc::clone(&data));
    match orchestrator.get_user_sessions(user_id).await {
        Ok(sessions) => {
            info!("Retrieved {} sessions", sessions.len());
            Ok(HttpResponse::Ok().json(sessions))
        }
        Err(e) => {
            error!("Failed to get sessions: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()})))
        }
    }
}

#[actix_web::get("/api/sessions/{session_id}")]
async fn get_session_history(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let session_id = path.into_inner();
    info!("Getting session history for: {}", session_id);

    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    match Uuid::parse_str(&session_id) {
        Ok(session_uuid) => {
            let orchestrator = BotOrchestrator::new(Arc::clone(&data));
            match orchestrator
                .get_conversation_history(session_uuid, user_id)
                .await
            {
                Ok(history) => {
                    info!(
                        "Retrieved {} history entries for session {}",
                        history.len(),
                        session_id
                    );
                    Ok(HttpResponse::Ok().json(history))
                }
                Err(e) => {
                    error!("Failed to get session history for {}: {}", session_id, e);
                    Ok(HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error": e.to_string()})))
                }
            }
        }
        Err(_) => {
            warn!("Invalid session ID format: {}", session_id);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid session ID"})))
        }
    }
}

#[actix_web::post("/api/set_mode")]
async fn set_mode_handler(
    data: web::Data<AppState>,
    info: web::Json<HashMap<String, String>>,
) -> Result<HttpResponse> {
    info!("Setting user answer mode");

    let default_user = "default_user".to_string();
    let default_bot = "default_bot".to_string();
    let default_mode = "0".to_string();

    let user_id = info.get("user_id").unwrap_or(&default_user);
    let bot_id = info.get("bot_id").unwrap_or(&default_bot);
    let mode_str = info.get("mode").unwrap_or(&default_mode);

    let mode = mode_str.parse::<i32>().unwrap_or(0);

    debug!(
        "Setting mode - user: {}, bot: {}, mode: {}",
        user_id, bot_id, mode
    );

    let orchestrator = BotOrchestrator::new(Arc::clone(&data));
    if let Err(e) = orchestrator
        .set_user_answer_mode(user_id, bot_id, mode)
        .await
    {
        error!("Failed to set answer mode: {}", e);
        return Ok(
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        );
    }

    info!("Answer mode updated successfully");
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "mode_updated"})))
}

#[actix_web::post("/api/warn")]
async fn send_warning_handler(
    data: web::Data<AppState>,
    info: web::Json<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let default_session = "default".to_string();
    let default_channel = "web".to_string();
    let default_message = "Warning!".to_string();

    let session_id = info.get("session_id").unwrap_or(&default_session);
    let channel = info.get("channel").unwrap_or(&default_channel);
    let message = info.get("message").unwrap_or(&default_message);

    info!(
        "Sending warning via API - session: {}, channel: {}",
        session_id, channel
    );

    let orchestrator = BotOrchestrator::new(Arc::clone(&data));
    if let Err(e) = orchestrator
        .send_warning(session_id, channel, message)
        .await
    {
        error!("Failed to send warning: {}", e);
        return Ok(
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        );
    }

    info!("Warning sent successfully");
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "warning_sent"})))
}

#[actix_web::get("/")]
async fn index() -> Result<HttpResponse> {
    info!("Serving index page");
    match fs::read_to_string("web/index.html") {
        Ok(html) => {
            debug!("Index page loaded successfully");
            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        Err(e) => {
            error!("Failed to load index page: {}", e);
            Ok(HttpResponse::InternalServerError().body("Failed to load index page"))
        }
    }
}

#[actix_web::get("/static/{filename:.*}")]
async fn static_files(req: HttpRequest) -> Result<HttpResponse> {
    let filename = req.match_info().query("filename");
    let path = format!("static/{}", filename);

    info!("Serving static file: {}", filename);

    match fs::read(&path) {
        Ok(content) => {
            debug!(
                "Static file {} loaded successfully, size: {} bytes",
                filename,
                content.len()
            );
            let content_type = match filename {
                f if f.ends_with(".js") => "application/javascript",
                f if f.ends_with(".css") => "text/css",
                f if f.ends_with(".png") => "image/png",
                f if f.ends_with(".jpg") | f.ends_with(".jpeg") => "image/jpeg",
                _ => "text/plain",
            };

            Ok(HttpResponse::Ok().content_type(content_type).body(content))
        }
        Err(e) => {
            warn!("Static file not found: {} - {}", filename, e);
            Ok(HttpResponse::NotFound().body("File not found"))
        }
    }
}
