use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use futures::{SinkExt, StreamExt};
use log::{error, info};
use serde::Deserialize;
use std::collections::HashMap;
use serde_json::Value;
use std::sync::Arc;

use botcore::urls::ApiUrls;
use botcore::shared::state::AppState;

pub mod transcription;
pub mod minutes;
pub mod conversations;
pub mod conversation_types;
pub mod conversation_store;
pub mod conversation_messages;
pub mod conversation_calls;
pub mod dashboard;
pub mod recording;
pub mod recording_routes;
pub mod room_persistence;
pub mod service;
pub mod signaling;
pub mod turn;
pub mod turn_config;
pub mod ui;
pub mod webinar;
pub mod webinar_api;
pub mod webinar_types;
pub mod whiteboard;
pub mod whiteboard_export;
pub mod trunks;

use room_persistence::ScheduleMeetingRequest;
use service::{DefaultTranscriptionService, MeetingService};
use signaling::SignalingRooms;

#[derive(Debug, Deserialize)]
pub struct CreateMeetingRequest {
    pub name: String,
    pub created_by: String,
    pub settings: Option<service::MeetingSettings>,
}

#[derive(Debug, Deserialize)]
pub struct JoinRoomRequest {
    pub participant_name: String,
    pub participant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LeaveRoomRequest {
    pub participant_id: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTokenRequest {
    pub room_id: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SendInvitesRequest {
    pub room_id: String,
    pub emails: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MeetJoinHtmxRequest {
    pub user_name: Option<String>,
    pub meeting_code: Option<String>,
    pub room_id: Option<String>,
    pub join_video: Option<String>,
    pub join_audio: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MuteAllRequest {
    pub room_id: Option<String>,
}

mod lib_part1;
mod lib_part2;
mod lib_part3;
mod lib_part4;

pub use lib_part1::*;
pub use lib_part2::*;
pub use lib_part3::*;
pub use lib_part4::*;
