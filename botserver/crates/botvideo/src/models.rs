use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::*;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = video_projects)]
pub struct VideoProject {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub resolution_width: i32,
    pub resolution_height: i32,
    pub fps: i32,
    pub total_duration_ms: i64,
    pub timeline_json: serde_json::Value,
    pub layers_json: serde_json::Value,
    pub audio_tracks_json: serde_json::Value,
    pub playhead_ms: i64,
    pub selection_json: serde_json::Value,
    pub zoom_level: f32,
    pub thumbnail_url: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = video_clips)]
pub struct VideoClip {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub source_url: String,
    pub start_ms: i64,
    pub duration_ms: i64,
    pub trim_in_ms: i64,
    pub trim_out_ms: i64,
    pub volume: f32,
    pub clip_order: i32,
    pub transition_in: Option<String>,
    pub transition_out: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = video_layers)]
pub struct VideoLayer {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub layer_type: String,
    pub track_index: i32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub opacity: f32,
    pub properties_json: serde_json::Value,
    pub animation_in: Option<String>,
    pub animation_out: Option<String>,
    pub locked: bool,
    pub created_at: DateTime<Utc>,
    pub keyframes_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = video_audio_tracks)]
pub struct VideoAudioTrack {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub source_url: String,
    pub track_type: String,
    pub start_ms: i64,
    pub duration_ms: i64,
    pub volume: f32,
    pub fade_in_ms: i64,
    pub fade_out_ms: i64,
    pub created_at: DateTime<Utc>,
    pub waveform_json: Option<serde_json::Value>,
    pub beat_markers_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = video_exports)]
pub struct VideoExport {
    pub id: Uuid,
    pub project_id: Uuid,
    pub format: String,
    pub quality: String,
    pub status: String,
    pub progress: i32,
    pub output_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub gbdrive_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = video_command_history)]
pub struct VideoCommandHistory {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Option<Uuid>,
    pub command_type: String,
    pub command_json: serde_json::Value,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = video_analytics)]
pub struct VideoAnalytics {
    pub id: Uuid,
    pub project_id: Uuid,
    pub export_id: Option<Uuid>,
    pub views: i64,
    pub unique_viewers: i64,
    pub total_watch_time_ms: i64,
    pub avg_watch_percent: f32,
    pub completions: i64,
    pub shares: i64,
    pub likes: i64,
    pub engagement_score: f32,
    pub viewer_retention_json: Option<serde_json::Value>,
    pub geography_json: Option<serde_json::Value>,
    pub device_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = video_keyframes)]
pub struct VideoKeyframe {
    pub id: Uuid,
    pub layer_id: Uuid,
    pub property_name: String,
    pub time_ms: i64,
    pub value_json: serde_json::Value,
    pub easing: String,
    pub created_at: DateTime<Utc>,
}
