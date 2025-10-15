use crate::channels::ChannelAdapter;
use crate::shared::models::{BotResponse, UserMessage, UserSession};
use crate::shared::state::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_ws::Message as WsMessage;
use chrono::Utc;
use log::{debug, error, info, warn};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct BotOrchestrator {
    pub state: Arc<AppState>,
}

impl BotOrchestrator {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn handle_user_input(
        &self,
        session_id: Uuid,
        user_input: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Handling user input for session {}: '{}'",
            session_id, user_input
        );
        let mut session_manager = self.state.session_manager.lock().await;
        session_manager.provide_input(session_id, user_input.to_string())?;
        Ok(None)
    }

    pub async fn is_waiting_for_input(&self, session_id: Uuid) -> bool {
        let session_manager = self.state.session_manager.lock().await;
        session_manager.is_waiting_for_input(&session_id)
    }

    pub fn add_channel(&self, channel_type: &str, adapter: Arc<dyn ChannelAdapter>) {
        self.state
            .channels
            .lock()
            .unwrap()
            .insert(channel_type.to_string(), adapter);
    }

    pub async fn register_response_channel(
        &self,
        session_id: String,
        sender: mpsc::Sender<BotResponse>,
    ) {
        self.state
            .response_channels
            .lock()
            .await
            .insert(session_id.clone(), sender);
    }

    pub async fn unregister_response_channel(&self, session_id: &str) {
        self.state.response_channels.lock().await.remove(session_id);
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
        info!(
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

        if let Some(adapter) = self.state.channels.lock().unwrap().get(channel) {
            adapter.send_message(event_response).await?;
        } else {
            warn!("No channel adapter found for channel 1: {}", channel);
        }
        Ok(())
    }

    pub async fn send_direct_message(
        &self,
        session_id: &str,
        channel: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Sending direct message to session {}: '{}'",
            session_id, content
        );
        let bot_response = BotResponse {
            bot_id: "default_bot".to_string(),
            user_id: "default_user".to_string(),
            session_id: session_id.to_string(),
            channel: channel.to_string(),
            content: content.to_string(),
            message_type: 1,
            stream_token: None,
            is_complete: true,
        };

        if let Some(adapter) = self.state.channels.lock().unwrap().get(channel) {
            adapter.send_message(bot_response).await?;
        } else {
            warn!("No channel adapter found for channel 2: {}", channel);
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

        let user_id = Uuid::parse_str(&message.user_id).map_err(|e| {
            error!("Invalid user ID provided: {}", e);
            e
        })?;

        let bot_id = if let Ok(bot_guid) = std::env::var("BOT_GUID") {
            Uuid::parse_str(&bot_guid).map_err(|e| {
                warn!("Invalid BOT_GUID from env: {}", e);
                e
            })?
        } else {
            warn!("BOT_GUID not set in environment, using nil UUID");
            Uuid::nil()
        };

        let session = {
            let mut sm = self.state.session_manager.lock().await;
            let session_id = Uuid::parse_str(&message.session_id).map_err(|e| {
                error!("Invalid session ID: {}", e);
                e
            })?;
            match sm.get_session_by_id(session_id)? {
                Some(session) => session,
                None => {
                    error!(
                        "Failed to create session for user {} with bot {}",
                        user_id, bot_id
                    );
                    return Err("Failed to create session".into());
                }
            }
        };

        if self.is_waiting_for_input(session.id).await {
            debug!(
                "Session {} is waiting for input, processing as variable input",
                session.id
            );
            if let Some(variable_name) =
                self.handle_user_input(session.id, &message.content).await?
            {
                debug!(
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
                }
            }
            return Ok(());
        }

        if session.answer_mode == 1 && session.current_tool.is_some() {
            self.state.tool_manager.provide_user_response(
                &message.user_id,
                &message.bot_id,
                message.content.clone(),
            )?;
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
        }

        let response_content = self.direct_mode_handler(&message, &session).await?;

        {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.save_message(session.id, user_id, 2, &response_content, 1)?;
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

        if let Some(adapter) = self.state.channels.lock().unwrap().get(&message.channel) {
            adapter.send_message(bot_response).await?;
        } else {
            warn!(
                "No channel adapter found for channel 3: {}",
                message.channel
            );
        }

        Ok(())
    }

    async fn direct_mode_handler(
        &self,
        message: &UserMessage,
        session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let system_prompt = std::env::var("SYSTEM_PROMPT").unwrap_or_default();
        let context_data = {
            let session_manager = self.state.session_manager.lock().await;
            session_manager
                .get_session_context(&session.id, &session.user_id)
                .await?
        };

        let mut prompt = String::new();
        if !system_prompt.is_empty() {
            prompt.push_str(&format!("System: {}\n", system_prompt));
        }
        if !context_data.is_empty() {
            prompt.push_str(&format!("Context: {}\n", context_data));
        }

        let history = {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.get_conversation_history(session.id, session.user_id)?
        };

        for (role, content) in history {
            prompt.push_str(&format!("{}: {}\n", role, content));
        }
        prompt.push_str(&format!("User: {}\nAssistant:", message.content));

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

        let user_id = Uuid::parse_str(&message.user_id).map_err(|e| {
            error!("Invalid user ID: {}", e);
            e
        })?;

        let _bot_id = if let Ok(bot_guid) = std::env::var("BOT_GUID") {
            Uuid::parse_str(&bot_guid).map_err(|e| {
                warn!("Invalid BOT_GUID from env: {}", e);
                e
            })?
        } else {
            warn!("BOT_GUID not set in environment, using nil UUID");
            Uuid::nil()
        };

        let session = {
            let mut sm = self.state.session_manager.lock().await;
            let session_id = Uuid::parse_str(&message.session_id).map_err(|e| {
                error!("Invalid session ID: {}", e);
                e
            })?;
            match sm.get_session_by_id(session_id)? {
                Some(sess) => sess,
                None => {
                    error!("Failed to create session for streaming");
                    return Err("Failed to create session".into());
                }
            }
        };

        if session.answer_mode == 1 && session.current_tool.is_some() {
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
        }

        let system_prompt = std::env::var("SYSTEM_PROMPT").unwrap_or_default();
        let context_data = {
            let session_manager = self.state.session_manager.lock().await;
            session_manager
                .get_session_context(&session.id, &session.user_id)
                .await?
        };

        let prompt = {
            let mut sm = self.state.session_manager.lock().await;
            let history = sm.get_conversation_history(session.id, user_id)?;
            let mut p = String::new();

            if !system_prompt.is_empty() {
                p.push_str(&format!("System: {}\n", system_prompt));
            }
            if !context_data.is_empty() {
                p.push_str(&format!("Context: {}\n", context_data));
            }

            for (role, content) in &history {
                p.push_str(&format!("{}: {}\n", role, content));
            }
            p.push_str(&format!("User: {}\nAssistant:", message.content));

            debug!(
                "Stream prompt constructed with {} history entries",
                history.len()
            );
            p
        };

        let (stream_tx, mut stream_rx) = mpsc::channel::<String>(100);
        let llm = self.state.llm_provider.clone();

        if message.channel == "web" {
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

        tokio::spawn(async move {
            if let Err(e) = llm
                .generate_stream(&prompt, &serde_json::Value::Null, stream_tx)
                .await
            {
                error!("LLM streaming error: {}", e);
            }
        });

        let mut full_response = String::new();
        let mut analysis_buffer = String::new();
        let mut in_analysis = false;
        let mut chunk_count = 0;
        let mut first_word_received = false;

        while let Some(chunk) = stream_rx.recv().await {
            chunk_count += 1;

            if !first_word_received && !chunk.trim().is_empty() {
                first_word_received = true;
                debug!("First word received in stream: '{}'", chunk);
            }

            analysis_buffer.push_str(&chunk);
            if analysis_buffer.contains("<|channel|>") && !in_analysis {
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

        {
            let mut sm = self.state.session_manager.lock().await;
            sm.save_message(session.id, user_id, 2, &full_response, 1)?;
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

        Ok(())
    }

    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        let mut session_manager = self.state.session_manager.lock().await;
        let sessions = session_manager.get_user_sessions(user_id)?;
        Ok(sessions)
    }

    pub async fn get_conversation_history(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Getting conversation history for session {} user {}",
            session_id, user_id
        );
        let mut session_manager = self.state.session_manager.lock().await;
        let history = session_manager.get_conversation_history(session_id, user_id)?;
        Ok(history)
    }

    pub async fn run_start_script(
        session: &UserSession,
        state: Arc<AppState>,
        token: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Running start script for session: {} with token: {:?}",
            session.id, token
        );
        let start_script_path = "./templates/annoucements.gbai/annoucements.gbdialog/start.bas";
        let start_script = match std::fs::read_to_string(start_script_path) {
            Ok(content) => content,
            Err(_) => r#"TALK "Welcome to General Bots!""#.to_string(),
        };
        debug!(
            "Start script content for session {}: {}",
            session.id, start_script
        );

        let session_clone = session.clone();
        let state_clone = state.clone();
        let script_service = crate::basic::ScriptService::new(state_clone, session_clone.clone());

        if let Some(_token_id_value) = token {}

        match script_service
            .compile(&start_script)
            .and_then(|ast| script_service.run(&ast))
        {
            Ok(result) => {
                info!(
                    "Start script executed successfully for session {}, result: {}",
                    session_clone.id, result
                );
                Ok(true)
            }
            Err(e) => {
                error!(
                    "Failed to run start script for session {}: {}",
                    session_clone.id, e
                );
                Ok(false)
            }
        }
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

    pub async fn trigger_auto_welcome(
        &self,
        session_id: &str,
        user_id: &str,
        _bot_id: &str,
        token: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Triggering auto welcome for user: {}, session: {}, token: {:?}",
            user_id, session_id, token
        );
        let session_uuid = Uuid::parse_str(session_id).map_err(|e| {
            error!("Invalid session ID: {}", e);
            e
        })?;

        let session = {
            let mut session_manager = self.state.session_manager.lock().await;
            match session_manager.get_session_by_id(session_uuid)? {
                Some(session) => session,
                None => {
                    error!("Failed to create session for auto welcome");
                    return Ok(false);
                }
            }
        };

        let result = Self::run_start_script(&session, Arc::clone(&self.state), token).await?;
        info!(
            "Auto welcome completed for session: {} with result: {}",
            session_id, result
        );
        Ok(result)
    }

    async fn get_web_response_channel(
        &self,
        session_id: &str,
    ) -> Result<mpsc::Sender<BotResponse>, Box<dyn std::error::Error + Send + Sync>> {
        let response_channels = self.state.response_channels.lock().await;
        if let Some(tx) = response_channels.get(session_id) {
            Ok(tx.clone())
        } else {
            Err("No response channel found for session".into())
        }
    }
}

impl Default for BotOrchestrator {
    fn default() -> Self {
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
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let session_id = query.get("session_id").cloned().unwrap();
    let user_id = query
        .get("user_id")
        .cloned()
        .unwrap_or_else(|| "default_user".to_string());

    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    let (tx, mut rx) = mpsc::channel::<BotResponse>(100);
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

    let bot_id = std::env::var("BOT_GUID").unwrap_or_else(|_| "default_bot".to_string());

    orchestrator
        .send_event(
            &user_id,
            &bot_id,
            &session_id,
            "web",
            "session_start",
            serde_json::json!({
                "session_id": session_id,
                "user_id": user_id,
                "timestamp": Utc::now().to_rfc3339()
            }),
        )
        .await
        .ok();

    info!(
        "WebSocket connection established for session: {}, user: {}",
        session_id, user_id
    );

    let web_adapter = data.web_adapter.clone();
    let session_id_clone1 = session_id.clone();
    let session_id_clone2 = session_id.clone();
    let user_id_clone = user_id.clone();

    actix_web::rt::spawn(async move {
        info!(
            "Starting WebSocket sender for session {}",
            session_id_clone1
        );
        let mut message_count = 0;
        while let Some(msg) = rx.recv().await {
            message_count += 1;
            if let Ok(json) = serde_json::to_string(&msg) {
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
                    let bot_id =
                        std::env::var("BOT_GUID").unwrap_or_else(|_| "default_bot".to_string());

                    // Parse the text as JSON to extract the content field
                    let json_value: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(value) => value,
                        Err(e) => {
                            error!("Error parsing JSON message {}: {}", message_count, e);
                            continue; // Skip processing this message
                        }
                    };

                    // Extract content from JSON, fallback to original text if content field doesn't exist
                    let content = json_value["content"]
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap();

                    let user_message = UserMessage {
                        bot_id: bot_id,
                        user_id: user_id_clone.clone(),
                        session_id: session_id_clone2.clone(),
                        channel: "web".to_string(),
                        content: content,
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
                    let bot_id =
                        std::env::var("BOT_GUID").unwrap_or_else(|_| "default_bot".to_string());
                    orchestrator
                        .send_event(
                            &user_id_clone,
                            &bot_id,
                            &session_id_clone2,
                            "web",
                            "session_end",
                            serde_json::json!({}),
                        )
                        .await
                        .ok();
                    web_adapter.remove_connection(&session_id_clone2).await;
                    orchestrator
                        .unregister_response_channel(&session_id_clone2)
                        .await;
                    break;
                }
                _ => {}
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

#[actix_web::get("/api/auth")]
async fn auth_handler(
    data: web::Data<AppState>,
    web::Query(params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let _token = params.get("token").cloned().unwrap_or_default();
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    let bot_id = if let Ok(bot_guid) = std::env::var("BOT_GUID") {
        match Uuid::parse_str(&bot_guid) {
            Ok(uuid) => uuid,
            Err(e) => {
                warn!("Invalid BOT_GUID from env: {}", e);
                return Ok(HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "Invalid BOT_GUID"})));
            }
        }
    } else {
        warn!("BOT_GUID not set in environment, using nil UUID");
        Uuid::nil()
    };

    let session = {
        let mut sm = data.session_manager.lock().await;
        match sm.get_or_create_user_session(user_id, bot_id, "Auth Session") {
            Ok(Some(s)) => s,
            Ok(None) => {
                error!("Failed to create session");
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to create session"})));
            }
            Err(e) => {
                error!("Failed to create session: {}", e);
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": e.to_string()})));
            }
        }
    };

    let session_id_clone = session.id.clone();
    let auth_script_path = "./templates/annoucements.gbai/annoucements.gbdialog/auth.bas";
    let auth_script = match std::fs::read_to_string(auth_script_path) {
        Ok(content) => content,
        Err(_) => r#"SET_USER "00000000-0000-0000-0000-000000000001""#.to_string(),
    };

    let script_service = crate::basic::ScriptService::new(Arc::clone(&data), session.clone());
    match script_service
        .compile(&auth_script)
        .and_then(|ast| script_service.run(&ast))
    {
        Ok(result) => {
            if result.to_string() == "false" {
                error!("Auth script returned false, authentication failed");
                return Ok(HttpResponse::Unauthorized()
                    .json(serde_json::json!({"error": "Authentication failed"})));
            }
        }
        Err(e) => {
            error!("Failed to run auth script: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Auth failed"})));
        }
    }

    let session = {
        let mut sm = data.session_manager.lock().await;
        match sm.get_session_by_id(session_id_clone) {
            Ok(Some(s)) => s,
            Ok(None) => {
                error!("Failed to retrieve session");
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to retrieve session"})));
            }
            Err(e) => {
                error!("Failed to retrieve session: {}", e);
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": e.to_string()})));
            }
        }
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user_id": session.user_id,
        "session_id": session.id,
        "status": "authenticated"
    })))
}

#[actix_web::get("/api/whatsapp/webhook")]
async fn whatsapp_webhook_verify(
    data: web::Data<AppState>,
    web::Query(params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let empty = String::new();
    let mode = params.get("hub.mode").unwrap_or(&empty);
    let token = params.get("hub.verify_token").unwrap_or(&empty);
    let challenge = params.get("hub.challenge").unwrap_or(&empty);
    info!(
        "Verification params - mode: {}, token: {}, challenge: {}",
        mode, token, challenge
    );

    match data.whatsapp_adapter.verify_webhook(mode, token, challenge) {
        Ok(challenge_response) => Ok(HttpResponse::Ok().body(challenge_response)),
        Err(_) => {
            warn!("WhatsApp webhook verification failed");
            Ok(HttpResponse::Forbidden().body("Verification failed"))
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

#[actix_web::post("/api/start")]
async fn start_session(
    data: web::Data<AppState>,
    info: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let session_id = info
        .get("session_id")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let token = info
        .get("token")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    let session_uuid = match Uuid::parse_str(session_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            warn!("Invalid session ID format: {}", session_id);
            return Ok(
                HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid session ID"}))
            );
        }
    };

    let session = {
        let mut session_manager = data.session_manager.lock().await;
        match session_manager.get_session_by_id(session_uuid) {
            Ok(Some(s)) => s,
            Ok(None) => {
                warn!("Session not found: {}", session_uuid);
                return Ok(HttpResponse::NotFound()
                    .json(serde_json::json!({"error": "Session not found"})));
            }
            Err(e) => {
                error!("Error retrieving session {}: {}", session_uuid, e);
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to retrieve session"})));
            }
        }
    };

    let result = BotOrchestrator::run_start_script(&session, Arc::clone(&data), token).await;
    match result {
        Ok(true) => {
            info!(
                "Start script completed successfully for session: {}",
                session_id
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "started",
                "session_id": session.id,
                "result": "success"
            })))
        }
        Ok(false) => {
            warn!("Start script returned false for session: {}", session_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "started",
                "session_id": session.id,
                "result": "failed"
            })))
        }
        Err(e) => {
            error!(
                "Error running start script for session {}: {}",
                session_id, e
            );
            Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()})))
        }
    }
}

#[actix_web::post("/api/sessions")]
async fn create_session(data: web::Data<AppState>) -> Result<HttpResponse> {
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let bot_id = if let Ok(bot_guid) = std::env::var("BOT_GUID") {
        match Uuid::parse_str(&bot_guid) {
            Ok(uuid) => uuid,
            Err(e) => {
                warn!("Invalid BOT_GUID from env: {}", e);
                return Ok(HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "Invalid BOT_GUID"})));
            }
        }
    } else {
        warn!("BOT_GUID not set in environment, using nil UUID");
        Uuid::nil()
    };

    let session = {
        let mut session_manager = data.session_manager.lock().await;
        match session_manager.get_or_create_user_session(user_id, bot_id, "New Conversation") {
            Ok(Some(s)) => s,
            Ok(None) => {
                error!("Failed to create session");
                return Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to create session"})));
            }
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
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let orchestrator = BotOrchestrator::new(Arc::clone(&data));
    match orchestrator.get_user_sessions(user_id).await {
        Ok(sessions) => Ok(HttpResponse::Ok().json(sessions)),
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
    let default_user = "default_user".to_string();
    let default_bot = "default_bot".to_string();
    let default_mode = "0".to_string();
    let user_id = info.get("user_id").unwrap_or(&default_user);
    let bot_id = info.get("bot_id").unwrap_or(&default_bot);
    let mode_str = info.get("mode").unwrap_or(&default_mode);
    let mode = mode_str.parse::<i32>().unwrap_or(0);

    info!(
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

    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "warning_sent"})))
}

#[actix_web::get("/")]
async fn index() -> Result<HttpResponse> {
    match fs::read_to_string("web/index.html") {
        Ok(html) => Ok(HttpResponse::Ok().content_type("text/html").body(html)),
        Err(e) => {
            error!("Failed to load index page: {}", e);
            Ok(HttpResponse::InternalServerError().body("Failed to load index page"))
        }
    }
}

#[actix_web::get("/static/{filename:.*}")]
async fn static_files(req: HttpRequest) -> Result<HttpResponse> {
    let filename = req.match_info().query("filename");
    let path = format!("web/static/{}", filename);
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
