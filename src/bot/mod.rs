use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_ws::Message as WsMessage;
use chrono::Utc;
use log::info;
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
        Self { state }
    }

    pub async fn handle_user_input(
        &self,
        session_id: Uuid,
        user_input: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
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
            .insert(session_id, sender);
    }

    pub async fn set_user_answer_mode(
        &self,
        user_id: &str,
        bot_id: &str,
        mode: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut session_manager = self.state.session_manager.lock().await;
        session_manager.update_answer_mode(user_id, bot_id, mode)?;
        Ok(())
    }

    pub async fn process_message(
        &self,
        message: UserMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Processing message from channel: {}, user: {}",
            message.channel, message.user_id
        );

        let user_id = Uuid::parse_str(&message.user_id).unwrap_or_else(|_| Uuid::new_v4());
        let bot_id = Uuid::parse_str(&message.bot_id)
            .unwrap_or_else(|_| Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());

        let session = {
            let mut session_manager = self.state.session_manager.lock().await;
            match session_manager.get_user_session(user_id, bot_id)? {
                Some(session) => session,
                None => {
                    let new_session =
                        session_manager.create_session(user_id, bot_id, "New Conversation")?;
                    Self::run_start_script(&new_session, Arc::clone(&self.state)).await;
                    new_session
                }
            }
        };

        if self.is_waiting_for_input(session.id).await {
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
                }
                return Ok(());
            }
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
        }

        Ok(())
    }

    async fn direct_mode_handler(
        &self,
        message: &UserMessage,
        session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let history = {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.get_conversation_history(session.id, session.user_id)?
        };

        let mut prompt = String::new();
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
        info!("Streaming response for user: {}", message.user_id);

        let mut user_id = Uuid::parse_str(&message.user_id).unwrap_or_else(|_| Uuid::new_v4());
        let bot_id = Uuid::parse_str(&message.bot_id).unwrap_or_else(|_| Uuid::nil());
        let mut auth = self.state.auth_service.lock().await;
        let user_exists = auth.get_user_by_id(user_id)?;

        if user_exists.is_none() {
            user_id = auth.create_user("anonymous1", "anonymous@local", "password")?;
        } else {
            user_id = user_exists.unwrap().id;
        }

        let session = {
            let mut sm = self.state.session_manager.lock().await;
            match sm.get_user_session(user_id, bot_id)? {
                Some(sess) => sess,
                None => {
                    let new_session = sm.create_session(user_id, bot_id, "New Conversation")?;
                    Self::run_start_script(&new_session, Arc::clone(&self.state)).await;
                    new_session
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

        let prompt = {
            let mut sm = self.state.session_manager.lock().await;
            let history = sm.get_conversation_history(session.id, user_id)?;
            let mut p = String::new();
            for (role, content) in history {
                p.push_str(&format!("{}: {}\n", role, content));
            }
            p.push_str(&format!("User: {}\nAssistant:", message.content));
            p
        };

        let (stream_tx, mut stream_rx) = mpsc::channel::<String>(100);
        let llm = self.state.llm_provider.clone();

        tokio::spawn(async move {
            if let Err(e) = llm
                .generate_stream(&prompt, &serde_json::Value::Null, stream_tx)
                .await
            {
                log::error!("LLM streaming error: {}", e);
            }
        });

        let mut full_response = String::new();
        while let Some(chunk) = stream_rx.recv().await {
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
                break;
            }
        }

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
        session_manager.get_user_sessions(user_id)
    }

    pub async fn get_conversation_history(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        let mut session_manager = self.state.session_manager.lock().await;
        session_manager.get_conversation_history(session_id, user_id)
    }

    pub async fn process_message_with_tools(
        &self,
        message: UserMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Processing message with tools from user: {}",
            message.user_id
        );

        let user_id = Uuid::parse_str(&message.user_id).unwrap_or_else(|_| Uuid::new_v4());
        let bot_id = Uuid::parse_str(&message.bot_id)
            .unwrap_or_else(|_| Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());

        let session = {
            let mut session_manager = self.state.session_manager.lock().await;
            match session_manager.get_user_session(user_id, bot_id)? {
                Some(session) => session,
                None => {
                    let new_session =
                        session_manager.create_session(user_id, bot_id, "New Conversation")?;
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
        }

        let is_tool_waiting = self
            .state
            .tool_manager
            .is_tool_waiting(&message.session_id)
            .await
            .unwrap_or(false);

        if is_tool_waiting {
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
                for output in tool_output {
                    let bot_response = BotResponse {
                        bot_id: message.bot_id.clone(),
                        user_id: message.user_id.clone(),
                        session_id: message.session_id.clone(),
                        channel: message.channel.clone(),
                        content: output,
                        message_type: 1,
                        stream_token: None,
                        is_complete: true,
                    };

                    if let Some(adapter) = self.state.channels.lock().unwrap().get(&message.channel)
                    {
                        adapter.send_message(bot_response).await?;
                    }
                }
            }
            return Ok(());
        }

        let response = if message.content.to_lowercase().contains("calculator")
            || message.content.to_lowercase().contains("calculate")
            || message.content.to_lowercase().contains("math")
        {
            match self
                .state
                .tool_manager
                .execute_tool("calculator", &message.session_id, &message.user_id)
                .await
            {
                Ok(tool_result) => {
                    let mut session_manager = self.state.session_manager.lock().await;
                    session_manager.save_message(session.id, user_id, 2, &tool_result.output, 2)?;

                    tool_result.output
                }
                Err(e) => {
                    format!("I encountered an error starting the calculator: {}", e)
                }
            }
        } else {
            let available_tools = self.state.tool_manager.list_tools();
            let tools_context = if !available_tools.is_empty() {
                format!("\n\nAvailable tools: {}. If the user needs calculations, suggest using the calculator tool.", available_tools.join(", "))
            } else {
                String::new()
            };

            let full_prompt = format!("{}{}", message.content, tools_context);

            self.state
                .llm_provider
                .generate(&full_prompt, &serde_json::Value::Null)
                .await?
        };

        {
            let mut session_manager = self.state.session_manager.lock().await;
            session_manager.save_message(session.id, user_id, 2, &response, 1)?;
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
        }

        Ok(())
    }

    async fn run_start_script(session: &UserSession, state: Arc<AppState>) {
        let start_script = r#"
TALK "Welcome to General Bots!"
HEAR name
TALK "Hello, " + name

text = GET "default.pdf"
SET CONTEXT text

resume = LLM "Build a resume from " + text
"#;

        info!("Running start.bas for session: {}", session.id);

        let session_clone = session.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            let state_for_run = state_clone.clone();
            if let Err(e) = crate::basic::ScriptService::new(state_clone, session_clone.clone())
                .compile(start_script)
                .and_then(|ast| {
                    crate::basic::ScriptService::new(state_for_run, session_clone.clone()).run(&ast)
                })
            {
                log::error!("Failed to run start.bas: {}", e);
            }
        });
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
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    let session_id = Uuid::new_v4().to_string();
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

    let web_adapter = data.web_adapter.clone();

    actix_web::rt::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = session.text(json).await;
            }
        }
    });

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.recv().await {
            match msg {
                WsMessage::Text(text) => {
                    let user_message = UserMessage {
                        bot_id: "default_bot".to_string(),
                        user_id: "default_user".to_string(),
                        session_id: session_id.clone(),
                        channel: "web".to_string(),
                        content: text.to_string(),
                        message_type: 1,
                        media_url: None,
                        timestamp: Utc::now(),
                    };

                    if let Err(e) = orchestrator.stream_response(user_message, tx.clone()).await {
                        info!("Error processing message: {}", e);
                    }
                }
                WsMessage::Close(_) => {
                    web_adapter.remove_connection(&session_id).await;
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(res)
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

    match data.whatsapp_adapter.verify_webhook(mode, token, challenge) {
        Ok(challenge_response) => Ok(HttpResponse::Ok().body(challenge_response)),
        Err(_) => Ok(HttpResponse::Forbidden().body("Verification failed")),
    }
}

#[actix_web::post("/api/whatsapp/webhook")]
async fn whatsapp_webhook(
    data: web::Data<AppState>,
    payload: web::Json<crate::whatsapp::WhatsAppMessage>,
) -> Result<HttpResponse> {
    match data
        .whatsapp_adapter
        .process_incoming_message(payload.into_inner())
        .await
    {
        Ok(user_messages) => {
            for user_message in user_messages {
                let orchestrator = BotOrchestrator::new(Arc::clone(&data));
                if let Err(e) = orchestrator.process_message(user_message).await {
                    log::error!("Error processing WhatsApp message: {}", e);
                }
            }
            Ok(HttpResponse::Ok().body(""))
        }
        Err(e) => {
            log::error!("Error processing WhatsApp webhook: {}", e);
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

    match data
        .voice_adapter
        .start_voice_session(session_id, user_id)
        .await
    {
        Ok(token) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({"token": token, "status": "started"})))
        }
        Err(e) => {
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
        Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({"status": "stopped"}))),
        Err(e) => {
            Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()})))
        }
    }
}

#[actix_web::post("/api/sessions")]
async fn create_session(_data: web::Data<AppState>) -> Result<HttpResponse> {
    let session_id = Uuid::new_v4();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "session_id": session_id,
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
                Ok(history) => Ok(HttpResponse::Ok().json(history)),
                Err(e) => Ok(HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": e.to_string()}))),
            }
        }
        Err(_) => {
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

    let orchestrator = BotOrchestrator::new(Arc::clone(&data));
    if let Err(e) = orchestrator
        .set_user_answer_mode(user_id, bot_id, mode)
        .await
    {
        return Ok(
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        );
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "mode_updated"})))
}

#[actix_web::get("/")]
async fn index() -> Result<HttpResponse> {
    let html = fs::read_to_string("web/index.html").unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[actix_web::get("/static/{filename:.*}")]
async fn static_files(req: HttpRequest) -> Result<HttpResponse> {
    let filename = req.match_info().query("filename");
    let path = format!("static/{}", filename);

    match fs::read(&path) {
        Ok(content) => {
            let content_type = match filename {
                f if f.ends_with(".js") => "application/javascript",
                f if f.ends_with(".css") => "text/css",
                f if f.ends_with(".png") => "image/png",
                f if f.ends_with(".jpg") | f.ends_with(".jpeg") => "image/jpeg",
                _ => "text/plain",
            };

            Ok(HttpResponse::Ok().content_type(content_type).body(content))
        }
        Err(_) => Ok(HttpResponse::NotFound().body("File not found")),
    }
}
