use super::*;

impl RecordingService {
    pub fn new(pool: DbPool, config: RecordingConfig) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        Self {
            pool,
            config,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            transcription_jobs: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RecordingEvent> {
        self.event_sender.subscribe()
    }

    pub async fn start_recording(
        &self,
        request: StartRecordingRequest,
    ) -> Result<WebinarRecording, RecordingError> {
        // Check if webinar is already being recorded
        let sessions = self.active_sessions.read().await;
        if sessions.values().any(|s| s.webinar_id == request.webinar_id) {
            return Err(RecordingError::AlreadyRecording);
        }
        drop(sessions);

        let recording_id = Uuid::new_v4();
        let now = Utc::now();
        let quality = request.quality.unwrap_or(self.config.default_quality.clone());

        // Create recording session
        let session = RecordingSession {
            id: recording_id,
            webinar_id: request.webinar_id,
            status: RecordingStatus::Recording,
            quality: quality.clone(),
            started_at: now,
            paused_at: None,
            total_paused_duration_ms: 0,
            audio_track_id: None,
            video_track_id: None,
            screen_share_track_id: None,
            file_path: Some(format!(
                "{}/{}/{}.webm",
                self.config.storage_bucket,
                request.webinar_id,
                recording_id
            )),
            chunk_count: 0,
            bytes_written: 0,
        };

        // Store session
        let mut sessions = self.active_sessions.write().await;
        sessions.insert(recording_id, session);
        drop(sessions);

        // Create database record
        self.create_recording_record(recording_id, request.webinar_id, &quality, now)
            .await?;

        // Broadcast event
        let _ = self.event_sender.send(RecordingEvent::Started {
            recording_id,
            webinar_id: request.webinar_id,
        });

        Ok(WebinarRecording {
            id: recording_id,
            webinar_id: request.webinar_id,
            status: RecordingStatus::Recording,
            duration_seconds: 0,
            file_size_bytes: 0,
            file_url: None,
            download_url: None,
            quality,
            started_at: now,
            ended_at: None,
            processed_at: None,
            expires_at: if self.config.retention_days > 0 {
                Some(now + chrono::Duration::days(self.config.retention_days as i64))
            } else {
                None
            },
            view_count: 0,
            download_count: 0,
        })
    }

    pub async fn pause_recording(&self, recording_id: Uuid) -> Result<(), RecordingError> {
        let mut sessions = self.active_sessions.write().await;
        let session = sessions
            .get_mut(&recording_id)
            .ok_or(RecordingError::NotFound)?;

        if session.status != RecordingStatus::Recording {
            return Err(RecordingError::InvalidState(
                "Recording is not active".to_string(),
            ));
        }

        session.paused_at = Some(Utc::now());

        let _ = self.event_sender.send(RecordingEvent::Paused { recording_id });

        Ok(())
    }

    pub async fn resume_recording(&self, recording_id: Uuid) -> Result<(), RecordingError> {
        let mut sessions = self.active_sessions.write().await;
        let session = sessions
            .get_mut(&recording_id)
            .ok_or(RecordingError::NotFound)?;

        if let Some(paused_at) = session.paused_at {
            let pause_duration = (Utc::now() - paused_at).num_milliseconds() as u64;
            session.total_paused_duration_ms += pause_duration;
            session.paused_at = None;
        }

        let _ = self.event_sender.send(RecordingEvent::Resumed { recording_id });

        Ok(())
    }

    pub async fn stop_recording(
        &self,
        request: StopRecordingRequest,
    ) -> Result<WebinarRecording, RecordingError> {
        let mut sessions = self.active_sessions.write().await;
        let session = sessions
            .remove(&request.recording_id)
            .ok_or(RecordingError::NotFound)?;
        drop(sessions);

        let now = Utc::now();
        let duration_seconds =
            ((now - session.started_at).num_milliseconds() as u64 - session.total_paused_duration_ms)
                / 1000;

        // Update database record
        self.update_recording_stopped(
            request.recording_id,
            now,
            duration_seconds,
            session.bytes_written,
        )
        .await?;

        // Broadcast stop event
        let _ = self.event_sender.send(RecordingEvent::Stopped {
            recording_id: request.recording_id,
            duration_seconds,
        });

        // Start processing
        self.process_recording(request.recording_id).await?;

        // Start transcription if requested
        if request.start_transcription.unwrap_or(self.config.auto_transcribe) {
            self.start_transcription(request.recording_id, session.webinar_id, None)
                .await?;
        }

        Ok(WebinarRecording {
            id: request.recording_id,
            webinar_id: session.webinar_id,
            status: RecordingStatus::Processing,
            duration_seconds,
            file_size_bytes: session.bytes_written,
            file_url: None,
            download_url: None,
            quality: session.quality,
            started_at: session.started_at,
            ended_at: Some(now),
            processed_at: None,
            expires_at: if self.config.retention_days > 0 {
                Some(now + chrono::Duration::days(self.config.retention_days as i64))
            } else {
                None
            },
            view_count: 0,
            download_count: 0,
        })
    }

    async fn process_recording(&self, recording_id: Uuid) -> Result<(), RecordingError> {
        let _ = self
            .event_sender
            .send(RecordingEvent::ProcessingStarted { recording_id });

        // In production, this would:
        // 1. Convert raw recording to final format (MP4/WebM)
        // 2. Generate multiple quality versions
        // 3. Generate thumbnails
        // 4. Upload to cloud storage
        // 5. Update database with URLs

        // The recording object is streamed by the server from MinIO drive at
        // `meet/recordings/{recording_id}.webm`; the file/download URLs point
        // at the authenticated server routes (see recording_routes.rs) so the
        // client never touches storage credentials directly.
        let file_url = format!("/api/meet/recordings/{recording_id}/file");
        let download_url = format!("/api/meet/recordings/{recording_id}/file");
        self.update_recording_processed(recording_id, &file_url, &download_url)
            .await?;

        let _ = self
            .event_sender
            .send(RecordingEvent::ProcessingCompleted {
                recording_id,
                file_url,
            });

        Ok(())
    }

    pub async fn start_transcription(
        &self,
        recording_id: Uuid,
        webinar_id: Uuid,
        language: Option<String>,
    ) -> Result<WebinarTranscription, RecordingError> {
        let transcription_id = Uuid::new_v4();
        let now = Utc::now();
        let language = language.unwrap_or_else(|| self.config.default_language.clone());

        // Validate language
        if !SUPPORTED_LANGUAGES.contains(&language.as_str()) {
            return Err(RecordingError::UnsupportedLanguage(language));
        }

        // Create transcription job
        let job = TranscriptionJob {
            id: transcription_id,
            recording_id,
            webinar_id,
            status: TranscriptionStatus::Pending,
            language: language.clone(),
            provider: self.config.transcription_provider.clone(),
            enable_speaker_diarization: self.config.speaker_diarization,
            created_at: now,
            started_at: None,
            completed_at: None,
            progress_percent: 0,
            error_message: None,
            retry_count: 0,
        };

        let mut jobs = self.transcription_jobs.write().await;
        jobs.insert(transcription_id, job);
        drop(jobs);

        // Create database record
        self.create_transcription_record(transcription_id, recording_id, &language)
            .await?;

        // Start transcription process (async)
        let service = self.clone_for_task();
        tokio::spawn(async move {
            service
                .run_transcription(transcription_id, recording_id)
                .await
        });

        let _ = self
            .event_sender
            .send(RecordingEvent::TranscriptionStarted {
                transcription_id,
                recording_id,
            });

        Ok(WebinarTranscription {
            id: transcription_id,
            webinar_id,
            recording_id,
            status: TranscriptionStatus::Pending,
            language,
            duration_seconds: 0,
            word_count: 0,
            speaker_count: 0,
            segments: vec![],
            full_text: None,
            vtt_url: None,
            srt_url: None,
            json_url: None,
            created_at: now,
            completed_at: None,
            confidence_score: 0.0,
        })
    }
}
