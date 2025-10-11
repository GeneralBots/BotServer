use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_ws::Message as WsMessage;
use chrono::Utc;
use log::info;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::auth::AuthService;
use crate::channels::ChannelAdapter;
use crate::llm::LLMProvider;
use crate::session::SessionManager;
use crate::shared::models::{BotResponse, UserMessage, UserSession};
use crate::tools::ToolManager;

pub struct BotOrchestrator {
    pub session_manager: Arc<Mutex<SessionManager>>,
    tool_manager: Arc<ToolManager>,
    llm_provider: Arc<dyn LLMProvider>,
    auth_service: AuthService,
    pub channels: HashMap<String, Arc<dyn ChannelAdapter>>,
    response_channels: Arc<Mutex<HashMap<String, mpsc::Sender<BotResponse>>>>,
}

impl BotOrchestrator {
    pub fn new(
        session_manager: SessionManager,
        tool_manager: ToolManager,
        llm_provider: Arc<dyn LLMProvider>,
        auth_service: AuthService,
    ) -> Self {
        Self {
            session_manager: Arc::new(Mutex::new(session_manager)),
            tool_manager: Arc::new(tool_manager),
            llm_provider,
            auth_service,
            channels: HashMap::new(),
            response_channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn handle_user_input(
        &self,
        session_id: Uuid,
        user_input: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let session_manager = self.session_manager.lock().await;
        session_manager.provide_input(session_id, user_input).await
    }

    pub async fn is_waiting_for_input(&self, session_id: Uuid) -> bool {
        let session_manager = self.session_manager.lock().await;
        session_manager.is_waiting_for_input(session_id).await
    }

    pub fn add_channel(&mut self, channel_type: &str, adapter: Arc<dyn ChannelAdapter>) {
        self.channels.insert(channel_type.to_string(), adapter);
    }

    pub async fn register_response_channel(
        &self,
        session_id: String,
        sender: mpsc::Sender<BotResponse>,
    ) {
        self.response_channels
            .lock()
            .await
            .insert(session_id, sender);
    }

    pub async fn set_user_answer_mode(
        &self,
        user_id: &str,
        bot_id: &str,
        mode: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut session_manager = self.session_manager.lock().await;
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
            let mut session_manager = self.session_manager.lock().await;
            match session_manager.get_user_session(user_id, bot_id)? {
                Some(session) => session,
                None => session_manager.create_session(user_id, bot_id, "New Conversation")?,
            }
        };

        // Check if we're waiting for HEAR input
        if self.is_waiting_for_input(session.id).await {
            if let Some(variable_name) =
                self.handle_user_input(session.id, &message.content).await?
            {
                info!(
                    "Stored user input in variable '{}' for session {}",
                    variable_name, session.id
                );

                // Send acknowledgment
                if let Some(adapter) = self.channels.get(&message.channel) {
                    let ack_response = BotResponse {
                        bot_id: message.bot_id.clone(),
                        user_id: message.user_id.clone(),
                        session_id: message.session_id.clone(),
                        channel: message.channel.clone(),
                        content: format!("Input stored in '{}'", variable_name),
                        message_type: "system".to_string(),
                        stream_token: None,
                        is_complete: true,
                    };
                    adapter.send_message(ack_response).await?;
                }
                return Ok(());
            }
        }

        if session.answer_mode == "tool" && session.current_tool.is_some() {
            self.tool_manager.provide_user_response(
                &message.user_id,
                &message.bot_id,
                message.content.clone(),
            )?;
            return Ok(());
        }

        {
            let mut session_manager = self.session_manager.lock().await;
            session_manager.save_message(
                session.id,
                user_id,
                "user",
                &message.content,
                &message.message_type,
            )?;
        }

        let response_content = self.direct_mode_handler(&message, &session).await?;

        {
            let mut session_manager = self.session_manager.lock().await;
            session_manager.save_message(
                session.id,
                user_id,
                "assistant",
                &response_content,
                "text",
            )?;
        }

        let bot_response = BotResponse {
            bot_id: message.bot_id,
            user_id: message.user_id,
            session_id: message.session_id.clone(),
            channel: message.channel.clone(),
            content: response_content,
            message_type: "text".to_string(),
            stream_token: None,
            is_complete: true,
        };

        if let Some(adapter) = self.channels.get(&message.channel) {
            adapter.send_message(bot_response).await?;
        }

        Ok(())
    }

    async fn direct_mode_handler(
        &self,
        message: &UserMessage,
        session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut session_manager = self.session_manager.lock().await;
        let history = session_manager.get_conversation_history(session.id, session.user_id)?;

        let mut prompt = String::new();
        for (role, content) in history {
            prompt.push_str(&format!("{}: {}\n", role, content));
        }
        prompt.push_str(&format!("User: {}\nAssistant:", message.content));

        self.llm_provider
            .generate(&prompt, &serde_json::Value::Null)
            .await
    }
    pub async fn stream_response(
        &self,
        message: UserMessage,
        response_tx: mpsc::Sender<BotResponse>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Streaming response for user: {}", message.user_id);

        let user_id = Uuid::parse_str(&message.user_id).unwrap_or_else(|_| Uuid::new_v4());
        let bot_id = Uuid::parse_str(&message.bot_id)
            .unwrap_or_else(|_| Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());

        let session = {
            let mut session_manager = self.session_manager.lock().await;
            match session_manager.get_user_session(user_id, bot_id)? {
                Some(session) => session,
                None => session_manager.create_session(user_id, bot_id, "New Conversation")?,
            }
        };

        if session.answer_mode == "tool" && session.current_tool.is_some() {
            self.tool_manager
                .provide_user_response(&message.user_id, &message.bot_id, message.content.clone())
                .await?;
            return Ok(());
        }

        {
            let mut session_manager = self.session_manager.lock().await;
            session_manager.save_message(
                session.id,
                user_id,
                "user",
                &message.content,
                &message.message_type,
            )?;
        }

        let history = {
            let session_manager = self.session_manager.lock().await;
            session_manager.get_conversation_history(session.id, user_id)?
        };

        let mut prompt = String::new();
        for (role, content) in history {
            prompt.push_str(&format!("{}: {}\n", role, content));
        }
        prompt.push_str(&format!("User: {}\nAssistant:", message.content));

        let (stream_tx, mut stream_rx) = mpsc::channel(100);
        let llm_provider = self.llm_provider.clone();
        let prompt_clone = prompt.clone();

        tokio::spawn(async move {
            let _ = llm_provider
                .generate_stream(&prompt_clone, &serde_json::Value::Null, stream_tx)
                .await;
        });

        let mut full_response = String::new();
        while let Some(chunk) = stream_rx.recv().await {
            full_response.push_str(&chunk);

            let bot_response = BotResponse {
                bot_id: message.bot_id.clone(),
                user_id: message.user_id.clone(),
                session_id: message.session_id.clone(),
                channel: message.channel.clone(),
                content: chunk,
                message_type: "text".to_string(),
                stream_token: None,
                is_complete: false,
            };

            if response_tx.send(bot_response).await.is_err() {
                break;
            }
        }

        {
            let mut session_manager = self.session_manager.lock().await;
            session_manager.save_message(
                session.id,
                user_id,
                "assistant",
                &full_response,
                "text",
            )?;
        }

        let final_response = BotResponse {
            bot_id: message.bot_id,
            user_id: message.user_id,
            session_id: message.session_id,
            channel: message.channel,
            content: "".to_string(),
            message_type: "text".to_string(),
            stream_token: None,
            is_complete: true,
        };

        response_tx.send(final_response).await?;
        Ok(())
    }

    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        let mut session_manager = self.session_manager.lock().await;
        session_manager.get_user_sessions(user_id)
    }

    pub async fn get_conversation_history(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        let mut session_manager = self.session_manager.lock().await;
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
            let mut session_manager = self.session_manager.lock().await;
            match session_manager.get_user_session(user_id, bot_id)? {
                Some(session) => session,
                None => session_manager.create_session(user_id, bot_id, "New Conversation")?,
            }
        };

        {
            let mut session_manager = self.session_manager.lock().await;
            session_manager.save_message(
                session.id,
                user_id,
                "user",
                &message.content,
                &message.message_type,
            )?;
        }

        let is_tool_waiting = self
            .tool_manager
            .is_tool_waiting(&message.session_id)
            .await
            .unwrap_or(false);

        if is_tool_waiting {
            self.tool_manager
                .provide_input(&message.session_id, &message.content)
                .await?;

            if let Ok(tool_output) = self.tool_manager.get_tool_output(&message.session_id).await {
                for output in tool_output {
                    let bot_response = BotResponse {
                        bot_id: message.bot_id.clone(),
                        user_id: message.user_id.clone(),
                        session_id: message.session_id.clone(),
                        channel: message.channel.clone(),
                        content: output,
                        message_type: "text".to_string(),
                        stream_token: None,
                        is_complete: true,
                    };

                    if let Some(adapter) = self.channels.get(&message.channel) {
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
                .tool_manager
                .execute_tool("calculator", &message.session_id, &message.user_id)
                .await
            {
                Ok(tool_result) => {
                    let mut session_manager = self.session_manager.lock().await;
                    session_manager.save_message(
                        session.id,
                        user_id,
                        "assistant",
                        &tool_result.output,
                        "tool_start",
                    )?;

                    tool_result.output
                }
                Err(e) => {
                    format!("I encountered an error starting the calculator: {}", e)
                }
            }
        } else {
            let available_tools = self.tool_manager.list_tools();
            let tools_context = if !available_tools.is_empty() {
                format!("\n\nAvailable tools: {}. If the user needs calculations, suggest using the calculator tool.", available_tools.join(", "))
            } else {
                String::new()
            };

            let full_prompt = format!("{}{}", message.content, tools_context);

            self.llm_provider
                .generate(&full_prompt, &serde_json::Value::Null)
                .await?
        };

        {
            let mut session_manager = self.session_manager.lock().await;
            session_manager.save_message(session.id, user_id, "assistant", &response, "text")?;
        }

        let bot_response = BotResponse {
            bot_id: message.bot_id,
            user_id: message.user_id,
            session_id: message.session_id.clone(),
            channel: message.channel.clone(),
            content: response,
            message_type: "text".to_string(),
            stream_token: None,
            is_complete: true,
        };

        if let Some(adapter) = self.channels.get(&message.channel) {
            adapter.send_message(bot_response).await?;
        }

        Ok(())
    }
}

#[actix_web::get("/ws")]
async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<crate::shared::state::AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    let session_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::channel::<BotResponse>(100);

    data.orchestrator
        .register_response_channel(session_id.clone(), tx.clone())
        .await;
    data.web_adapter
        .add_connection(session_id.clone(), tx.clone())
        .await;
    data.voice_adapter
        .add_connection(session_id.clone(), tx.clone())
        .await;

    let orchestrator = data.orchestrator.clone();
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
                        message_type: "text".to_string(),
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
    data: web::Data<crate::shared::state::AppState>,
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
    data: web::Data<crate::shared::state::AppState>,
    payload: web::Json<crate::whatsapp::WhatsAppMessage>,
) -> Result<HttpResponse> {
    match data
        .whatsapp_adapter
        .process_incoming_message(payload.into_inner())
        .await
    {
        Ok(user_messages) => {
            for user_message in user_messages {
                if let Err(e) = data.orchestrator.process_message(user_message).await {
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
    data: web::Data<crate::shared::state::AppState>,
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
    data: web::Data<crate::shared::state::AppState>,
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
async fn create_session(_data: web::Data<crate::shared::state::AppState>) -> Result<HttpResponse> {
    let session_id = Uuid::new_v4();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "session_id": session_id,
        "title": "New Conversation",
        "created_at": Utc::now()
    })))
}

#[actix_web::get("/api/sessions")]
async fn get_sessions(data: web::Data<crate::shared::state::AppState>) -> Result<HttpResponse> {
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    match data.orchestrator.get_user_sessions(user_id).await {
        Ok(sessions) => Ok(HttpResponse::Ok().json(sessions)),
        Err(e) => {
            Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()})))
        }
    }
}

#[actix_web::get("/api/sessions/{session_id}")]
async fn get_session_history(
    data: web::Data<crate::shared::state::AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let session_id = path.into_inner();
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    match Uuid::parse_str(&session_id) {
        Ok(session_uuid) => match data
            .orchestrator
            .get_conversation_history(session_uuid, user_id)
            .await
        {
            Ok(history) => Ok(HttpResponse::Ok().json(history)),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()}))),
        },
        Err(_) => {
            Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid session ID"})))
        }
    }
}

#[actix_web::post("/api/set_mode")]
async fn set_mode_handler(
    data: web::Data<crate::shared::state::AppState>,
    info: web::Json<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let default_user = "default_user".to_string();
    let default_bot = "default_bot".to_string();
    let default_mode = "direct".to_string();

    let user_id = info.get("user_id").unwrap_or(&default_user);
    let bot_id = info.get("bot_id").unwrap_or(&default_bot);
    let mode = info.get("mode").unwrap_or(&default_mode);

    if let Err(e) = data
        .orchestrator
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
    let html = fs::read_to_string("templates/index.html")
        .unwrap_or_else(|_| include_str!("../../static/index.html").to_string());
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
