use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_ws::Message as WsMessage;
use chrono::Utc;
use langchain_rust::{
    chain::{Chain, LLMChain},
    llm::openai::OpenAI,
    memory::SimpleMemory,
    prompt_args,
    tools::{postgres::PostgreSQLEngine, SQLDatabaseBuilder},
    vectorstore::qdrant::Qdrant as LangChainQdrant,
    vectorstore::{VecStoreOptions, VectorStore},
};
use log::info;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::{
    auth::AuthService,
    channels::{ChannelAdapter, VoiceAdapter, WebChannelAdapter},
    chart::ChartGenerator,
    llm::LLMProvider,
    session::SessionManager,
    shared::{BotResponse, UserMessage, UserSession},
    tools::ToolManager,
    whatsapp::WhatsAppAdapter,
};

pub struct BotOrchestrator {
    session_manager: SessionManager,
    tool_manager: ToolManager,
    llm_provider: Arc<dyn LLMProvider>,
    auth_service: AuthService,
    channels: HashMap<String, Arc<dyn ChannelAdapter>>,
    response_channels: Arc<Mutex<HashMap<String, mpsc::Sender<BotResponse>>>>,
    chart_generator: Option<Arc<ChartGenerator>>,
    vector_store: Option<Arc<LangChainQdrant>>,
    sql_chain: Option<Arc<LLMChain>>,
}

impl BotOrchestrator {
    pub fn new(
        session_manager: SessionManager,
        tool_manager: ToolManager,
        llm_provider: Arc<dyn LLMProvider>,
        auth_service: AuthService,
        chart_generator: Option<Arc<ChartGenerator>>,
        vector_store: Option<Arc<LangChainQdrant>>,
        sql_chain: Option<Arc<LLMChain>>,
    ) -> Self {
        Self {
            session_manager,
            tool_manager,
            llm_provider,
            auth_service,
            channels: HashMap::new(),
            response_channels: Arc::new(Mutex::new(HashMap::new())),
            chart_generator,
            vector_store,
            sql_chain,
        }
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
        self.session_manager
            .update_answer_mode(user_id, bot_id, mode)
            .await?;
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

        let session = match self
            .session_manager
            .get_user_session(user_id, bot_id)
            .await?
        {
            Some(session) => session,
            None => {
                self.session_manager
                    .create_session(user_id, bot_id, "New Conversation")
                    .await?
            }
        };

        if session.answer_mode == "tool" && session.current_tool.is_some() {
            self.tool_manager
                .provide_user_response(&message.user_id, &message.bot_id, message.content.clone())
                .await?;
            return Ok(());
        }

        self.session_manager
            .save_message(
                session.id,
                user_id,
                "user",
                &message.content,
                &message.message_type,
            )
            .await?;

        let response_content = match session.answer_mode.as_str() {
            "document" => self.document_mode_handler(&message, &session).await?,
            "chart" => self.chart_mode_handler(&message, &session).await?,
            "database" => self.database_mode_handler(&message, &session).await?,
            "tool" => self.tool_mode_handler(&message, &session).await?,
            _ => self.direct_mode_handler(&message, &session).await?,
        };

        self.session_manager
            .save_message(session.id, user_id, "assistant", &response_content, "text")
            .await?;

        let bot_response = BotResponse {
            bot_id: message.bot_id,
            user_id: message.user_id,
            session_id: message.session_id,
            channel: message.channel,
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

    async fn document_mode_handler(
        &self,
        message: &UserMessage,
        session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(vector_store) = &self.vector_store {
            let similar_docs = vector_store
                .similarity_search(&message.content, 3, &VecStoreOptions::default())
                .await?;

            let mut enhanced_prompt = format!("User question: {}\n\n", message.content);

            if !similar_docs.is_empty() {
                enhanced_prompt.push_str("Relevant documents:\n");
                for (i, doc) in similar_docs.iter().enumerate() {
                    enhanced_prompt.push_str(&format!("[Doc {}]: {}\n", i + 1, doc.page_content));
                }
                enhanced_prompt.push_str(
                    "\nPlease answer the user's question based on the provided documents.",
                );
            }

            self.llm_provider
                .generate(&enhanced_prompt, &serde_json::Value::Null)
                .await
        } else {
            self.direct_mode_handler(message, session).await
        }
    }

    async fn chart_mode_handler(
        &self,
        message: &UserMessage,
        session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(chart_generator) = &self.chart_generator {
            let chart_response = chart_generator
                .generate_chart(&message.content, "bar")
                .await?;

            self.session_manager
                .save_message(
                    session.id,
                    session.user_id,
                    "system",
                    &format!("Generated chart for query: {}", message.content),
                    "chart",
                )
                .await?;

            Ok(format!(
                "Chart generated for your query. Data retrieved: {}",
                chart_response.sql_query
            ))
        } else {
            self.document_mode_handler(message, session).await
        }
    }

    async fn database_mode_handler(
        &self,
        message: &UserMessage,
        _session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(sql_chain) = &self.sql_chain {
            let input_variables = prompt_args! {
                "input" => message.content,
            };

            let result = sql_chain.invoke(input_variables).await?;
            Ok(result.to_string())
        } else {
            let db_url = std::env::var("DATABASE_URL")?;
            let engine = PostgreSQLEngine::new(&db_url).await?;
            let db = SQLDatabaseBuilder::new(engine).build().await?;

            let llm = OpenAI::default();
            let chain = langchain_rust::chain::SQLDatabaseChainBuilder::new()
                .llm(llm)
                .top_k(5)
                .database(db)
                .build()?;

            let input_variables = chain.prompt_builder().query(&message.content).build();
            let result = chain.invoke(input_variables).await?;

            Ok(result.to_string())
        }
    }

    async fn tool_mode_handler(
        &self,
        message: &UserMessage,
        _session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if message.content.to_lowercase().contains("calculator") {
            if let Some(_adapter) = self.channels.get(&message.channel) {
                let (tx, _rx) = mpsc::channel(100);

                self.register_response_channel(message.session_id.clone(), tx.clone())
                    .await;

                let tool_manager = self.tool_manager.clone();
                let user_id_str = message.user_id.clone();
                let bot_id_str = message.bot_id.clone();
                let session_manager = self.session_manager.clone();

                tokio::spawn(async move {
                    let _ = tool_manager
                        .execute_tool_with_session(
                            "calculator",
                            &user_id_str,
                            &bot_id_str,
                            session_manager,
                            tx,
                        )
                        .await;
                });
            }
            Ok("Starting calculator tool...".to_string())
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
                .await
        }
    }

    async fn direct_mode_handler(
        &self,
        message: &UserMessage,
        session: &UserSession,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let history = self
            .session_manager
            .get_conversation_history(session.id, session.user_id)
            .await?;

        let mut memory = SimpleMemory::new();
        for (role, content) in history {
            match role.as_str() {
                "user" => memory.add_user_message(&content),
                "assistant" => memory.add_ai_message(&content),
                _ => {}
            }
        }

        let mut prompt = String::new();
        if let Some(chat_history) = memory.get_chat_history() {
            for message in chat_history {
                prompt.push_str(&format!(
                    "{}: {}\n",
                    message.message_type(),
                    message.content()
                ));
            }
        }
        prompt.push_str(&format!("User: {}\nAssistant:", message.content));

        self.llm_provider
            .generate(&prompt, &serde_json::Value::Null)
            .await
    }

    pub async fn stream_response(
        &self,
        message: UserMessage,
        mut response_tx: mpsc::Sender<BotResponse>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Streaming response for user: {}", message.user_id);

        let user_id = Uuid::parse_str(&message.user_id).unwrap_or_else(|_| Uuid::new_v4());
        let bot_id = Uuid::parse_str(&message.bot_id)
            .unwrap_or_else(|_| Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());

        let session = match self
            .session_manager
            .get_user_session(user_id, bot_id)
            .await?
        {
            Some(session) => session,
            None => {
                self.session_manager
                    .create_session(user_id, bot_id, "New Conversation")
                    .await?
            }
        };

        if session.answer_mode == "tool" && session.current_tool.is_some() {
            self.tool_manager
                .provide_user_response(&message.user_id, &message.bot_id, message.content.clone())
                .await?;
            return Ok(());
        }

        self.session_manager
            .save_message(
                session.id,
                user_id,
                "user",
                &message.content,
                &message.message_type,
            )
            .await?;

        let history = self
            .session_manager
            .get_conversation_history(session.id, user_id)
            .await?;

        let mut memory = SimpleMemory::new();
        for (role, content) in history {
            match role.as_str() {
                "user" => memory.add_user_message(&content),
                "assistant" => memory.add_ai_message(&content),
                _ => {}
            }
        }

        let mut prompt = String::new();
        if let Some(chat_history) = memory.get_chat_history() {
            for message in chat_history {
                prompt.push_str(&format!(
                    "{}: {}\n",
                    message.message_type(),
                    message.content()
                ));
            }
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

        self.session_manager
            .save_message(session.id, user_id, "assistant", &full_response, "text")
            .await?;

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
        self.session_manager.get_user_sessions(user_id).await
    }

    pub async fn get_conversation_history(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        self.session_manager
            .get_conversation_history(session_id, user_id)
            .await
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

        let session = match self
            .session_manager
            .get_user_session(user_id, bot_id)
            .await?
        {
            Some(session) => session,
            None => {
                self.session_manager
                    .create_session(user_id, bot_id, "New Conversation")
                    .await?
            }
        };

        self.session_manager
            .save_message(
                session.id,
                user_id,
                "user",
                &message.content,
                &message.message_type,
            )
            .await?;

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
                    self.session_manager
                        .save_message(
                            session.id,
                            user_id,
                            "assistant",
                            &tool_result.output,
                            "tool_start",
                        )
                        .await?;

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

        self.session_manager
            .save_message(session.id, user_id, "assistant", &response, "text")
            .await?;

        let bot_response = BotResponse {
            bot_id: message.bot_id,
            user_id: message.user_id,
            session_id: message.session_id,
            channel: message.channel,
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
    data: web::Data<crate::shared::AppState>,
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
    data: web::Data<crate::shared::AppState>,
    web::Query(params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let mode = params.get("hub.mode").unwrap_or(&"".to_string());
    let token = params.get("hub.verify_token").unwrap_or(&"".to_string());
    let challenge = params.get("hub.challenge").unwrap_or(&"".to_string());

    match data.whatsapp_adapter.verify_webhook(mode, token, challenge) {
        Ok(challenge_response) => Ok(HttpResponse::Ok().body(challenge_response)),
        Err(_) => Ok(HttpResponse::Forbidden().body("Verification failed")),
    }
}

#[actix_web::post("/api/whatsapp/webhook")]
async fn whatsapp_webhook(
    data: web::Data<crate::shared::AppState>,
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
    data: web::Data<crate::shared::AppState>,
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
    data: web::Data<crate::shared::AppState>,
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
async fn create_session(_data: web::Data<crate::shared::AppState>) -> Result<HttpResponse> {
    let session_id = Uuid::new_v4();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "session_id": session_id,
        "title": "New Conversation",
        "created_at": Utc::now()
    })))
}

#[actix_web::get("/api/sessions")]
async fn get_sessions(data: web::Data<crate::shared::AppState>) -> Result<HttpResponse> {
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
    data: web::Data<crate::shared::AppState>,
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
    data: web::Data<crate::shared::AppState>,
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
