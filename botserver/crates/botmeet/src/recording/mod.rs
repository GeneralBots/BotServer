pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::{broadcast, RwLock};
pub(crate) use uuid::Uuid;
pub(crate) use botschema::meeting_recordings;
pub(crate) use botcore::shared::utils::DbPool;
pub(crate) use botcore::shared::utils::{format_timestamp_plain, format_timestamp_srt, format_timestamp_vtt};

#[derive(Debug, Clone)]
pub enum RecordingError {
    DatabaseError(String),
    NotFound,
    AlreadyExists,
    AlreadyRecording,
    InvalidState(String),
    StorageError(String),
    TranscriptionError(String),
    TranscriptionNotReady,
    UnsupportedLanguage(String),
    ExportFailed(String),
    Unauthorized,
}

use crate::webinar_types::{
    RecordingQuality, RecordingStatus, TranscriptionFormat, TranscriptionSegment,
    TranscriptionStatus, TranscriptionWord, WebinarRecording, WebinarTranscription,
};

const MAX_RECORDING_DURATION_SECONDS: u64 = 28800;

const DEFAULT_TRANSCRIPTION_LANGUAGE: &str = "en-US";

const SUPPORTED_LANGUAGES: &[&str] = &[
    "en-US", "en-GB", "es-ES", "es-MX", "fr-FR", "de-DE", "it-IT", "pt-BR", "pt-PT", "nl-NL",
    "pl-PL", "ru-RU", "ja-JP", "ko-KR", "zh-CN", "zh-TW", "ar-SA", "hi-IN", "tr-TR", "vi-VN",
];


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub max_duration_seconds: u64,
    pub default_quality: RecordingQuality,
    pub storage_backend: StorageBackend,
    pub storage_bucket: String,
    pub auto_transcribe: bool,
    pub default_language: String,
    pub transcription_provider: TranscriptionProvider,
    pub retention_days: u32,
    pub speaker_diarization: bool,
    pub max_speakers: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum StorageBackend {
    #[default]
    Local,
    S3,
    Azure,
    Gcs,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TranscriptionProvider {
    #[default]
    Whisper,
    AzureSpeech,
    GoogleSpeech,
    AwsTranscribe,
    DeepGram,
    AssemblyAI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSession {
    pub id: Uuid,
    pub webinar_id: Uuid,
    pub status: RecordingStatus,
    pub quality: RecordingQuality,
    pub started_at: DateTime<Utc>,
    pub paused_at: Option<DateTime<Utc>>,
    pub total_paused_duration_ms: u64,
    pub audio_track_id: Option<String>,
    pub video_track_id: Option<String>,
    pub screen_share_track_id: Option<String>,
    pub file_path: Option<String>,
    pub chunk_count: u32,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionJob {
    pub id: Uuid,
    pub recording_id: Uuid,
    pub webinar_id: Uuid,
    pub status: TranscriptionStatus,
    pub language: String,
    pub provider: TranscriptionProvider,
    pub enable_speaker_diarization: bool,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress_percent: u8,
    pub error_message: Option<String>,
    pub retry_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordingEvent {
    Started {
        recording_id: Uuid,
        webinar_id: Uuid,
    },
    Paused {
        recording_id: Uuid,
    },
    Resumed {
        recording_id: Uuid,
    },
    Stopped {
        recording_id: Uuid,
        duration_seconds: u64,
    },
    ChunkWritten {
        recording_id: Uuid,
        chunk_number: u32,
        bytes: u64,
    },
    ProcessingStarted {
        recording_id: Uuid,
    },
    ProcessingCompleted {
        recording_id: Uuid,
        file_url: String,
    },
    ProcessingFailed {
        recording_id: Uuid,
        error: String,
    },
    TranscriptionStarted {
        transcription_id: Uuid,
        recording_id: Uuid,
    },
    TranscriptionProgress {
        transcription_id: Uuid,
        progress_percent: u8,
    },
    TranscriptionSegmentReady {
        transcription_id: Uuid,
        segment: TranscriptionSegment,
    },
    TranscriptionCompleted {
        transcription_id: Uuid,
        word_count: u32,
    },
    TranscriptionFailed {
        transcription_id: Uuid,
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRecordingRequest {
    pub webinar_id: Uuid,
    pub quality: Option<RecordingQuality>,
    pub enable_transcription: Option<bool>,
    pub transcription_language: Option<String>,
    pub speaker_diarization: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopRecordingRequest {
    pub recording_id: Uuid,
    pub start_transcription: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTranscriptionRequest {
    pub format: TranscriptionFormat,
    pub include_timestamps: bool,
    pub include_speaker_names: bool,
    pub max_line_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTranscriptionResponse {
    pub format: TranscriptionFormat,
    pub content: String,
    pub content_type: String,
    pub filename: String,
}

pub struct RecordingService {
    pool: DbPool,
    config: RecordingConfig,
    active_sessions: Arc<RwLock<HashMap<Uuid, RecordingSession>>>,
    transcription_jobs: Arc<RwLock<HashMap<Uuid, TranscriptionJob>>>,
    event_sender: broadcast::Sender<RecordingEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_status_display() {
        assert_eq!(format!("{}", RecordingStatus::Recording), "recording");
        assert_eq!(format!("{}", RecordingStatus::Processing), "processing");
        assert_eq!(format!("{}", RecordingStatus::Ready), "ready");
    }

    #[test]
    fn test_recording_quality_display() {
        assert_eq!(format!("{}", RecordingQuality::Standard), "standard");
        assert_eq!(format!("{}", RecordingQuality::High), "high");
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
