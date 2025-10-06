use async_trait::async_trait;
use chrono::Utc;
use livekit::{DataPacketKind, Room, RoomOptions};
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::shared::{BotResponse, UserMessage};

#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    async fn send_message(&self, response: BotResponse) -> Result<(), Box<dyn std::error::Error>>;
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
    async fn send_message(&self, response: BotResponse) -> Result<(), Box<dyn std::error::Error>> {
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
    rooms: Arc<Mutex<HashMap<String, Room>>>,
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
    ) -> Result<String, Box<dyn std::error::Error>> {
        let token = AccessToken::with_api_key(&self.api_key, &self.api_secret)
            .with_identity(user_id)
            .with_name(user_id)
            .with_room_name(session_id)
            .with_room_join(true)
            .to_jwt()?;

        let room_options = RoomOptions {
            auto_subscribe: true,
            ..Default::default()
        };

        let (room, mut events) = Room::connect(&self.livekit_url, &token, room_options).await?;
        self.rooms
            .lock()
            .await
            .insert(session_id.to_string(), room.clone());

        let rooms_clone = self.rooms.clone();
        let connections_clone = self.connections.clone();
        let session_id_clone = session_id.to_string();

        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                match event {
                    livekit::prelude::RoomEvent::DataReceived(data_packet) => {
                        if let Ok(message) =
                            serde_json::from_slice::<UserMessage>(&data_packet.data)
                        {
                            info!("Received voice message: {}", message.content);
                            if let Some(tx) =
                                connections_clone.lock().await.get(&message.session_id)
                            {
                                let _ = tx
                                    .send(BotResponse {
                                        bot_id: message.bot_id,
                                        user_id: message.user_id,
                                        session_id: message.session_id,
                                        channel: "voice".to_string(),
                                        content: format!("🎤 Voice: {}", message.content),
                                        message_type: "voice".to_string(),
                                        stream_token: None,
                                        is_complete: true,
                                    })
                                    .await;
                            }
                        }
                    }
                    livekit::prelude::RoomEvent::TrackSubscribed(
                        track,
                        publication,
                        participant,
                    ) => {
                        info!("Voice track subscribed from {}", participant.identity());
                    }
                    _ => {}
                }
            }
            rooms_clone.lock().await.remove(&session_id_clone);
        });

        Ok(token)
    }

    pub async fn stop_voice_session(
        &self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(room) = self.rooms.lock().await.remove(session_id) {
            room.disconnect();
        }
        Ok(())
    }

    pub async fn add_connection(&self, session_id: String, tx: mpsc::Sender<BotResponse>) {
        self.connections.lock().await.insert(session_id, tx);
    }

    pub async fn send_voice_response(
        &self,
        session_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(room) = self.rooms.lock().await.get(session_id) {
            let voice_response = serde_json::json!({
                "type": "voice_response",
                "text": text,
                "timestamp": Utc::now()
            });

            room.local_participant().publish_data(
                serde_json::to_vec(&voice_response)?,
                DataPacketKind::Reliable,
                &[],
            )?;
        }
        Ok(())
    }
}

#[async_trait]
impl ChannelAdapter for VoiceAdapter {
    async fn send_message(&self, response: BotResponse) -> Result<(), Box<dyn std::error::Error>> {
        info!("Sending voice response to: {}", response.user_id);
        self.send_voice_response(&response.session_id, &response.content)
            .await
    }
}
