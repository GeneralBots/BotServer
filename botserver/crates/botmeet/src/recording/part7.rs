use super::*;

/// Selected `meeting_recordings` columns in select order.
type RecordingRow = (
    Uuid,
    Uuid,
    String,
    Option<String>,
    Option<i64>,
    Option<i32>,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

impl RecordingService {
    pub(super) fn format_as_srt(
        &self,
        transcription: &WebinarTranscription,
        request: &ExportTranscriptionRequest,
    ) -> String {
        let mut output = String::new();

        for (i, segment) in transcription.segments.iter().enumerate() {
            output.push_str(&format!("{}\n", i + 1));
            output.push_str(&format!(
                "{} --> {}\n",
                format_timestamp_srt(segment.start_time_ms as i64),
                format_timestamp_srt(segment.end_time_ms as i64)
            ));

            let mut text = segment.text.clone();
            if request.include_speaker_names {
                if let Some(ref speaker) = segment.speaker_name {
                    text = format!("[{}] {}", speaker, text);
                }
            }
            output.push_str(&format!("{}\n\n", text));
        }

        output
    }

    pub async fn list_recordings(
        &self,
        webinar_id: Uuid,
    ) -> Result<Vec<WebinarRecording>, RecordingError> {
        self.list_recordings_from_db(webinar_id).await
    }

    pub async fn delete_recording(&self, recording_id: Uuid) -> Result<(), RecordingError> {
        // Check if recording is active
        let sessions = self.active_sessions.read().await;
        if sessions.contains_key(&recording_id) {
            return Err(RecordingError::InvalidState(
                "Cannot delete active recording".to_string(),
            ));
        }
        drop(sessions);

        // Delete from storage
        self.delete_recording_files(recording_id).await?;

        // Delete from database
        self.delete_recording_from_db(recording_id).await
    }

    // Database helper methods

    pub(crate) async fn get_recording_from_db(&self, recording_id: Uuid) -> Result<WebinarRecording, RecordingError> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            let row: RecordingRow = meeting_recordings::table
                .filter(meeting_recordings::id.eq(recording_id))
                .select((
                    meeting_recordings::id,
                    meeting_recordings::room_id,
                    meeting_recordings::recording_type,
                    meeting_recordings::file_url,
                    meeting_recordings::file_size,
                    meeting_recordings::duration_seconds,
                    meeting_recordings::status,
                    meeting_recordings::started_at,
                    meeting_recordings::stopped_at,
                    meeting_recordings::processed_at,
                ))
                .first(&mut conn)
                .map_err(|_| RecordingError::NotFound)?;

            let status = match row.6.as_str() {
                "recording" => RecordingStatus::Recording,
                "processing" => RecordingStatus::Processing,
                "ready" => RecordingStatus::Ready,
                "failed" => RecordingStatus::Failed,
                "deleted" => RecordingStatus::Deleted,
                _ => RecordingStatus::Failed,
            };

            let quality = match row.2.as_str() {
                "high" | "hd" => RecordingQuality::High,
                "low" | "audio" => RecordingQuality::AudioOnly,
                _ => RecordingQuality::Standard,
            };

            Ok(WebinarRecording {
                id: row.0,
                webinar_id: row.1,
                status,
                duration_seconds: row.5.unwrap_or(0) as u64,
                file_size_bytes: row.4.unwrap_or(0) as u64,
                file_url: row.3.clone(),
                download_url: row.3,
                quality,
                started_at: row.7,
                ended_at: row.8,
                processed_at: row.9,
                expires_at: None,
                view_count: 0,
                download_count: 0,
            })
        })
        .await
        .map_err(|e| RecordingError::DatabaseError(e.to_string()))?
    }

    async fn delete_recording_from_db(&self, recording_id: Uuid) -> Result<(), RecordingError> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            diesel::update(meeting_recordings::table.filter(meeting_recordings::id.eq(recording_id)))
                .set((
                    meeting_recordings::status.eq("deleted"),
                    meeting_recordings::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| RecordingError::DatabaseError(e.to_string()))?
    }

    async fn list_recordings_from_db(&self, room_id: Uuid) -> Result<Vec<WebinarRecording>, RecordingError> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            let rows: Vec<RecordingRow> = meeting_recordings::table
                .filter(meeting_recordings::room_id.eq(room_id))
                .filter(meeting_recordings::status.ne("deleted"))
                .order(meeting_recordings::started_at.desc())
                .select((
                    meeting_recordings::id,
                    meeting_recordings::room_id,
                    meeting_recordings::recording_type,
                    meeting_recordings::file_url,
                    meeting_recordings::file_size,
                    meeting_recordings::duration_seconds,
                    meeting_recordings::status,
                    meeting_recordings::started_at,
                    meeting_recordings::stopped_at,
                    meeting_recordings::processed_at,
                ))
                .load(&mut conn)
                .map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            let recordings = rows.into_iter().map(|row| {
                let status = match row.6.as_str() {
                    "recording" => RecordingStatus::Recording,
                    "processing" => RecordingStatus::Processing,
                    "ready" => RecordingStatus::Ready,
                    "failed" => RecordingStatus::Failed,
                    "deleted" => RecordingStatus::Deleted,
                    _ => RecordingStatus::Failed,
                };

                let quality = match row.2.as_str() {
                    "high" | "hd" => RecordingQuality::High,
                    "low" | "audio" => RecordingQuality::AudioOnly,
                    _ => RecordingQuality::Standard,
                };

                WebinarRecording {
                    id: row.0,
                    webinar_id: row.1,
                    status,
                    duration_seconds: row.5.unwrap_or(0) as u64,
                    file_size_bytes: row.4.unwrap_or(0) as u64,
                    file_url: row.3.clone(),
                    download_url: row.3,
                    quality,
                    started_at: row.7,
                    ended_at: row.8,
                    processed_at: row.9,
                    expires_at: None,
                    view_count: 0,
                    download_count: 0,
                }
            }).collect();

            Ok(recordings)
        })
        .await
        .map_err(|e| RecordingError::DatabaseError(e.to_string()))?
    }

    pub(crate) async fn create_recording_record(
        &self,
        recording_id: Uuid,
        webinar_id: Uuid,
        quality: &RecordingQuality,
        started_at: DateTime<Utc>,
    ) -> Result<(), RecordingError> {
        let pool = self.pool.clone();
        let quality_str = match quality {
            RecordingQuality::AudioOnly => "audio_only",
            RecordingQuality::Standard => "standard",
            RecordingQuality::High => "high",
            RecordingQuality::Ultra => "ultra",
        }.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            let branch_id = Uuid::nil();

            diesel::insert_into(meeting_recordings::table)
                .values((
                    meeting_recordings::id.eq(recording_id),
                    meeting_recordings::room_id.eq(webinar_id),
                    meeting_recordings::branch_id.eq(branch_id),
                    meeting_recordings::recording_type.eq(&quality_str),
                    meeting_recordings::status.eq("recording"),
                    meeting_recordings::started_at.eq(started_at),
                    meeting_recordings::metadata.eq(serde_json::json!({})),
                    meeting_recordings::created_at.eq(Utc::now()),
                    meeting_recordings::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| RecordingError::DatabaseError(e.to_string()))?
    }

    pub(crate) async fn update_recording_stopped(
        &self,
        recording_id: Uuid,
        ended_at: DateTime<Utc>,
        duration_seconds: u64,
        file_size_bytes: u64,
    ) -> Result<(), RecordingError> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            diesel::update(meeting_recordings::table.filter(meeting_recordings::id.eq(recording_id)))
                .set((
                    meeting_recordings::status.eq("processing"),
                    meeting_recordings::stopped_at.eq(ended_at),
                    meeting_recordings::duration_seconds.eq(duration_seconds as i32),
                    meeting_recordings::file_size.eq(file_size_bytes as i64),
                    meeting_recordings::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| RecordingError::DatabaseError(e.to_string()))?
    }

    pub(crate) async fn update_recording_processed(
        &self,
        recording_id: Uuid,
        file_url: &str,
        _download_url: &str,
    ) -> Result<(), RecordingError> {
        let pool = self.pool.clone();
        let file_url = file_url.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            diesel::update(meeting_recordings::table.filter(meeting_recordings::id.eq(recording_id)))
                .set((
                    meeting_recordings::status.eq("ready"),
                    meeting_recordings::file_url.eq(&file_url),
                    meeting_recordings::processed_at.eq(Utc::now()),
                    meeting_recordings::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(|e| RecordingError::DatabaseError(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| RecordingError::DatabaseError(e.to_string()))?
    }
}
