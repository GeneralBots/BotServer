use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use botcore::shared::state::AppState;
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::message::run_start_bas_on_connect;
use crate::core::bot::pipeline::{self, ChannelSink, PipelineError};

struct WebSocketSink(Arc<tokio::sync::Mutex<futures_util::stream::SplitSink<WebSocket, Message>>>);

#[async_trait::async_trait]
impl ChannelSink for WebSocketSink {
    async fn send_bot_response(&self, response: &botlib::models::BotResponse) -> Result<(), PipelineError> {
        if let Ok(json) = serde_json::to_string(response) {
            self.0.lock().await.send(Message::Text(json)).await
                .map_err(|e| PipelineError::Transport(format!("WS send: {e}")))?;
        }
        Ok(())
    }

    async fn send_raw_json(&self, json: &serde_json::Value) -> Result<(), PipelineError> {
        self.0.lock().await.send(Message::Text(json.to_string())).await
            .map_err(|e| PipelineError::Transport(format!("WS raw send: {e}")))?;
        Ok(())
    }

    async fn send_error(&self, session_id: &str, message: &str) -> Result<(), PipelineError> {
        let resp = botlib::models::BotResponse::new("", session_id, "", message, "web");
        self.send_bot_response(&resp).await
    }

    fn channel_type(&self) -> &str { "web" }
    fn supports_streaming(&self) -> bool { true }
    fn supports_suggestions(&self) -> bool { true }
    fn supports_raw_frames(&self) -> bool { true }
}

pub async fn handle_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    session_id: Uuid,
    user_id: Uuid,
    bot_uuid: Uuid,
    bot_name: String,
) {
    let (ws_sender, mut ws_receiver) = socket.split();
    let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));
    let (tx, mut rx) = mpsc::channel::<botlib::models::BotResponse>(100);
    let key = session_id.to_string();
    // Keep a handle so disconnect can prove ownership of the map entry.
    let tx_handle = tx.clone();
    {
        let mut channels = state.response_channels.lock().await;
        // #1288 — parallel chat tabs mean MULTIPLE sockets can carry the same
        // session_id. Overwriting the previous sender dropped it, ending that
        // socket's recv loop and closing it (stock tab vs parallel tab
        // flapping accept/close forever). First socket wins the entry: each
        // socket still receives replies to its OWN messages via its local rx;
        // the map only routes async/pushed frames.
        channels.entry(key.clone()).or_insert(tx);
    }
    info!("WebSocket connected: bot={bot}, session={session_id}", bot = bot_name);

    let welcome = serde_json::json!({
        "type": "connected", "session_id": session_id, "user_id": user_id,
        "bot_id": bot_uuid, "message": "Connected to bot server", "tools": []
    });
    let _ = ws_sender.lock().await.send(Message::Text(welcome.to_string())).await;

    {
        let mut pending = state.pending_stream_responses.lock().await;
        if let Some(content) = pending.remove(&session_id.to_string()) {
            info!("Delivering pending stream response for session {session_id} ({len} bytes)", len = content.len());
            let resp = serde_json::json!({
                "bot_id": bot_uuid.to_string(),
                "user_id": user_id.to_string(),
                "session_id": session_id.to_string(),
                "channel": "web",
                "content": content,
                "message_type": 2,
                "is_complete": true,
                "suggestions": [],
                "switchers": [],
                "context_length": 0,
                "context_max_length": 0,
            });
            let _ = ws_sender.lock().await.send(Message::Text(resp.to_string())).await;
        }
    }

    let mut start_bas_guard = ws_sender.lock().await;
    let mut start_bas_ran = run_start_bas_on_connect(
        &state, &mut start_bas_guard, &mut rx,
        bot_uuid, session_id, user_id, &bot_name,
    ).await;
    drop(start_bas_guard);

    loop {
        tokio::select! {
            response = rx.recv() => {
                if let Some(response) = response {
                    if let Ok(json) = serde_json::to_string(&response) {
                        if ws_sender.lock().await.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let sink = WebSocketSink(ws_sender.clone());
                        let _ = pipeline::process_message_internal(
                            &sink, &mut rx, &state,
                            session_id, user_id, bot_uuid, &bot_name,
                            &mut start_bas_ran, &text,
                        ).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_sender.lock().await.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => { error!("WS err: {e}"); break; }
                    _ => {}
                }
            }
        }
    }

    {
        let mut channels = state.response_channels.lock().await;
        // #1288 — remove only OUR registration: with parallel tabs another
        // socket may own this session key, and a blind remove would leave it
        // without an entry for pushed frames.
        if channels
            .get(&key)
            .is_some_and(|stored| stored.same_channel(&tx_handle))
        {
            channels.remove(&key);
        }
    }
    if let Ok(mut hear_map) = state.hear_channels.lock() {
        hear_map.remove(&session_id);
    }
    info!("WebSocket disconnected: session={session_id}");
}