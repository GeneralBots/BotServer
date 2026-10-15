pub(crate) use botcore::shared::{BotResponse, UserMessage};
pub(crate) use botcore::shared::state::AppState;
pub(crate) use anyhow::Result;
pub(crate) use async_trait::async_trait;
pub(crate) use axum::extract::ws::{Message, WebSocket};
pub(crate) use botlib::message_types::MessageType;
pub(crate) use futures::{SinkExt, StreamExt};
pub(crate) use log::{info, trace, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::{mpsc, RwLock};
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParticipantInfo {
    pub participant_id: String,
    pub participant_name: String,
    pub has_video: bool,
    pub has_audio: bool,
    pub is_screen_sharing: bool,
    pub is_hand_raised: bool,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignalingState {
    pub participants: HashMap<String, ParticipantInfo>,
    pub pending_offers: HashMap<String, serde_json::Value>,
    pub pending_candidates: HashMap<String, Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub role: ParticipantRole,
    pub is_bot: bool,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantRole {
    Host,
    Moderator,
    Participant,
    Bot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingRoom {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub participants: Vec<Participant>,
    pub is_recording: bool,
    pub is_transcribing: bool,
    pub max_participants: usize,
    pub settings: MeetingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSettings {
    pub enable_transcription: bool,
    pub enable_recording: bool,
    pub enable_chat: bool,
    pub enable_screen_share: bool,
    pub auto_admit: bool,
    pub waiting_room: bool,
    pub bot_enabled: bool,
    pub bot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MeetingMessage {
    JoinMeeting {
        room_id: String,
        participant_name: String,
        participant_id: Option<String>,
    },

    LeaveMeeting {
        room_id: String,
        participant_id: String,
    },

    Transcription {
        room_id: String,
        participant_id: String,
        text: String,
        timestamp: chrono::DateTime<chrono::Utc>,
        confidence: f32,
        is_final: bool,
    },

    ChatMessage {
        room_id: String,
        participant_id: String,
        content: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    BotMessage {
        room_id: String,
        content: String,
        in_response_to: Option<String>,
        metadata: HashMap<String, String>,
    },

    ScreenShare {
        room_id: String,
        participant_id: String,
        is_sharing: bool,
        share_type: Option<ScreenShareType>,
    },

    StatusUpdate {
        room_id: String,
        status: MeetingStatus,
        details: Option<String>,
    },

    ParticipantUpdate {
        room_id: String,
        participant: Participant,
        action: ParticipantAction,
    },

    RecordingControl {
        room_id: String,
        action: RecordingAction,
        participant_id: String,
    },

    BotRequest {
        room_id: String,
        participant_id: String,
        command: String,
        parameters: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenShareType {
    Screen,
    Window,
    Tab,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingStatus {
    Waiting,
    Active,
    Paused,
    Ended,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantAction {
    Joined,
    Left,
    Updated,
    Muted,
    Unmuted,
    VideoOn,
    VideoOff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingAction {
    Start,
    Stop,
    Pause,
    Resume,
}

pub struct MeetingService {
    pub state: Arc<AppState>,
    pub rooms: Arc<RwLock<HashMap<String, MeetingRoom>>>,
    pub connections: Arc<RwLock<HashMap<String, mpsc::Sender<MeetingMessage>>>>,
    pub transcription_service: Arc<dyn TranscriptionService + Send + Sync>,
}

#[async_trait]
pub trait TranscriptionService: Send + Sync {
    async fn start_transcription(&self, room_id: &str) -> Result<()>;
    async fn stop_transcription(&self, room_id: &str) -> Result<()>;
    async fn process_audio(&self, audio_data: Vec<u8>, room_id: &str) -> Result<String>;
}

#[derive(Debug)]
pub struct DefaultTranscriptionService;

mod part1;
mod part2;
mod part3;
