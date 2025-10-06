use async_trait::async_trait;
use chrono::Utc;
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::shared::{BotResponse, UserMessage};

#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    async fn send_message(&self, response: BotResponse) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct WebChannelAdapter {
    connections: Arc<Mutex<HashMap<String, mpsc::Sender<BotResponse>>>>,
}

impl WebChannelAdapter {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, session_id: String, tx: mpsc::Sender<BotResponse>) {
        self.connections.lock().await.insert(session_id, tx);
    }

    pub async fn remove_connection(&self, session_id: &str) {
        self.connections.lock().await.remove(session_id);
    }
}

#[async_trait]
impl ChannelAdapter for WebChannelAdapter {
    async fn send_message(&self, response: BotResponse) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connections = self.connections.lock().await;
        if let Some(tx) = connections.get(&response.session_id) {
            tx.send(response).await?;
        }
        Ok(())
    }
}

pub struct VoiceAdapter {
    livekit_url: String,
    api_key: String,
    api_secret: String,
    rooms: Arc<Mutex<HashMap<String, String>>>,
    connections: Arc<Mutex<HashMap<String, mpsc::Sender<BotResponse>>>>,
}

impl VoiceAdapter {
    pub fn new(livekit_url: String, api_key: String, api_secret: String) -> Self {
        Self {
            livekit_url,
            api_key,
            api_secret,
            rooms: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_voice_session(
        &self,
        session_id: &str,
        user_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting voice session for user: {} with session: {}", user_id, session_id);
        
        let token = format!("mock_token_{}_{}", session_id, user_id);
        self.rooms.lock().await.insert(session_id.to_string(), token.clone());
        
        Ok(token)
    }

    pub async fn stop_voice_session(
        &self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.rooms.lock().await.remove(session_id);
        Ok(())
    }

    pub async fn add_connection(&self, session_id: String, tx: mpsc::Sender<BotResponse>) {
        self.connections.lock().await.insert(session_id, tx);
    }

    pub async fn send_voice_response(
        &self,
        session_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Sending voice response to session {}: {}", session_id, text);
        Ok(())
    }
}

#[async_trait]
impl ChannelAdapter for VoiceAdapter {
    async fn send_message(&self, response: BotResponse) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Sending voice response to: {}", response.user_id);
        self.send_voice_response(&response.session_id, &response.content)
            .await
    }
}
