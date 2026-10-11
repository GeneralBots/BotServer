use super::*;

impl RecordingService {
    pub(crate) async fn create_transcription_record(
        &self,
        _transcription_id: Uuid,
        _recording_id: Uuid,
        _language: &str,
    ) -> Result<(), RecordingError> {
        // Transcription records use a separate table - implement when needed
        Ok(())
    }

    pub fn clone_for_task(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            transcription_jobs: Arc::new(RwLock::new(HashMap::new())),
            event_sender: self.event_sender.clone(),
        }
    }

    pub(crate) async fn update_transcription_completed(
        &self,
        _transcription_id: Uuid,
        _text: &str,
        _segments: &[TranscriptionSegment],
    ) -> Result<(), RecordingError> {
        Ok(())
    }

    pub(crate) async fn get_transcription_from_db(
        &self,
        _transcription_id: Uuid,
    ) -> Result<WebinarTranscription, RecordingError> {
        Err(RecordingError::NotFound)
    }

    pub(crate) async fn delete_recording_files(&self, _recording_id: Uuid) -> Result<(), RecordingError> {
        Ok(())
    }
}
