use super::*;

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            max_duration_seconds: MAX_RECORDING_DURATION_SECONDS,
            default_quality: RecordingQuality::Standard,
            storage_backend: StorageBackend::Local,
            storage_bucket: "recordings".to_string(),
            auto_transcribe: true,
            default_language: DEFAULT_TRANSCRIPTION_LANGUAGE.to_string(),
            transcription_provider: TranscriptionProvider::Whisper,
            retention_days: 90,
            speaker_diarization: true,
            max_speakers: 10,
        }
    }
}
