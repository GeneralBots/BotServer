use super::*;

#[async_trait]
impl TranscriptionService for DefaultTranscriptionService {
    async fn start_transcription(&self, room_id: &str) -> Result<()> {
        info!("Starting transcription for room: {}", room_id);
        Ok(())
    }

    async fn stop_transcription(&self, room_id: &str) -> Result<()> {
        info!("Stopping transcription for room: {}", room_id);
        Ok(())
    }

    async fn process_audio(&self, _audio_data: Vec<u8>, room_id: &str) -> Result<String> {
        Ok(format!("Transcribed text for room {}", room_id))
    }
}
