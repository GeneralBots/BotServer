use super::*;

impl RecordingService {
    pub(crate) async fn run_transcription(&self, transcription_id: Uuid, recording_id: Uuid) {
        log::info!("Starting transcription {transcription_id} for recording {recording_id}");
        // Update status to in progress
        {
            let mut jobs = self.transcription_jobs.write().await;
            if let Some(job) = jobs.get_mut(&transcription_id) {
                job.status = TranscriptionStatus::InProgress;
                job.started_at = Some(Utc::now());
            }
        }

        // In production, this would:
        // 1. Download/access the recording file
        // 2. Extract audio track
        // 3. Send to transcription provider (Whisper, Azure, etc.)
        // 4. Process results with speaker diarization
        // 5. Store segments in database
        // 6. Generate VTT/SRT files

        // Simulate transcription progress
        for progress in (0..=100).step_by(10) {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            {
                let mut jobs = self.transcription_jobs.write().await;
                if let Some(job) = jobs.get_mut(&transcription_id) {
                    job.progress_percent = progress as u8;
                }
            }

            let _ = self
                .event_sender
                .send(RecordingEvent::TranscriptionProgress {
                    transcription_id,
                    progress_percent: progress as u8,
                });

            // Emit segment at 50%
            if progress == 50 {
                let segment = TranscriptionSegment {
                    id: Uuid::new_v4(),
                    start_time_ms: 0,
                    end_time_ms: 5000,
                    text: "Welcome to this webinar session.".to_string(),
                    speaker_id: Some("speaker_1".to_string()),
                    speaker_name: Some("Host".to_string()),
                    confidence: 0.95,
                    words: vec![
                        TranscriptionWord {
                            word: "Welcome".to_string(),
                            start_time_ms: 0,
                            end_time_ms: 500,
                            confidence: 0.98,
                        },
                        TranscriptionWord {
                            word: "to".to_string(),
                            start_time_ms: 500,
                            end_time_ms: 700,
                            confidence: 0.99,
                        },
                        TranscriptionWord {
                            word: "this".to_string(),
                            start_time_ms: 700,
                            end_time_ms: 900,
                            confidence: 0.97,
                        },
                        TranscriptionWord {
                            word: "webinar".to_string(),
                            start_time_ms: 900,
                            end_time_ms: 1500,
                            confidence: 0.96,
                        },
                        TranscriptionWord {
                            word: "session".to_string(),
                            start_time_ms: 1500,
                            end_time_ms: 2000,
                            confidence: 0.94,
                        },
                    ],
                };

                let _ = self
                    .event_sender
                    .send(RecordingEvent::TranscriptionSegmentReady {
                        transcription_id,
                        segment,
                    });
            }
        }

        // Mark as completed
        {
            let mut jobs = self.transcription_jobs.write().await;
            if let Some(job) = jobs.get_mut(&transcription_id) {
                job.status = TranscriptionStatus::Completed;
                job.completed_at = Some(Utc::now());
                job.progress_percent = 100;
            }
        }

        // Create mock transcription data
        let full_text = "Welcome to this webinar session.".to_string();
        let segments: Vec<TranscriptionSegment> = vec![];

        // Update database
        let _ = self
            .update_transcription_completed(transcription_id, &full_text, &segments)
            .await;

        let _ = self
            .event_sender
            .send(RecordingEvent::TranscriptionCompleted {
                transcription_id,
                word_count: 1500,
            });
    }

    pub async fn get_recording(&self, recording_id: Uuid) -> Result<WebinarRecording, RecordingError> {
        // Check active sessions first
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(&recording_id) {
            let duration_seconds = if session.paused_at.is_some() {
                0 // Paused
            } else {
                ((Utc::now() - session.started_at).num_milliseconds() as u64
                    - session.total_paused_duration_ms)
                    / 1000
            };

            return Ok(WebinarRecording {
                id: session.id,
                webinar_id: session.webinar_id,
                status: session.status.clone(),
                duration_seconds,
                file_size_bytes: session.bytes_written,
                file_url: None,
                download_url: None,
                quality: session.quality.clone(),
                started_at: session.started_at,
                ended_at: None,
                processed_at: None,
                expires_at: None,
                view_count: 0,
                download_count: 0,
            });
        }
        drop(sessions);

        // Query database
        self.get_recording_from_db(recording_id).await
    }

    pub async fn get_transcription(
        &self,
        transcription_id: Uuid,
    ) -> Result<WebinarTranscription, RecordingError> {
        // Check active jobs first
        let jobs = self.transcription_jobs.read().await;
        if let Some(job) = jobs.get(&transcription_id) {
            return Ok(WebinarTranscription {
                id: job.id,
                webinar_id: job.webinar_id,
                recording_id: job.recording_id,
                status: job.status.clone(),
                language: job.language.clone(),
                duration_seconds: 0,
                word_count: 0,
                speaker_count: 0,
                segments: vec![],
                full_text: None,
                vtt_url: None,
                srt_url: None,
                json_url: None,
                created_at: job.created_at,
                completed_at: job.completed_at,
                confidence_score: 0.0,
            });
        }
        drop(jobs);

        // Query database
        self.get_transcription_from_db(transcription_id).await
    }

    pub async fn export_transcription(
        &self,
        transcription_id: Uuid,
        request: ExportTranscriptionRequest,
    ) -> Result<ExportTranscriptionResponse, RecordingError> {
        let transcription = self.get_transcription(transcription_id).await?;

        if transcription.status != TranscriptionStatus::Completed {
            return Err(RecordingError::TranscriptionNotReady);
        }

        let (content, content_type, extension) = match request.format {
            TranscriptionFormat::PlainText => {
                let text = self.format_as_plain_text(&transcription, &request);
                (text, "text/plain", "txt")
            }
            TranscriptionFormat::Vtt => {
                let vtt = self.format_as_vtt(&transcription, &request);
                (vtt, "text/vtt", "vtt")
            }
            TranscriptionFormat::Srt => {
                let srt = self.format_as_srt(&transcription, &request);
                (srt, "application/x-subrip", "srt")
            }
            TranscriptionFormat::Json => {
                let json = serde_json::to_string_pretty(&transcription)
                    .map_err(|e| RecordingError::ExportFailed(e.to_string()))?;
                (json, "application/json", "json")
            }
        };

        Ok(ExportTranscriptionResponse {
            format: request.format,
            content,
            content_type: content_type.to_string(),
            filename: format!("transcription_{}.{}", transcription_id, extension),
        })
    }

    pub(super) fn format_as_plain_text(
        &self,
        transcription: &WebinarTranscription,
        request: &ExportTranscriptionRequest,
    ) -> String {
        let mut output = String::new();

        for segment in &transcription.segments {
            if request.include_speaker_names {
                if let Some(ref speaker) = segment.speaker_name {
                    output.push_str(&format!("[{}] ", speaker));
                }
            }
            if request.include_timestamps {
                output.push_str(&format!(
                    "[{} - {}] ",
                    format_timestamp_plain(segment.start_time_ms as i64),
                    format_timestamp_plain(segment.end_time_ms as i64)
                ));
            }
            output.push_str(&segment.text);
            output.push('\n');
        }

        output
    }

    pub(super) fn format_as_vtt(
        &self,
        transcription: &WebinarTranscription,
        request: &ExportTranscriptionRequest,
    ) -> String {
        let mut output = String::from("WEBVTT\n\n");

        for (i, segment) in transcription.segments.iter().enumerate() {
            output.push_str(&format!("{}\n", i + 1));
            output.push_str(&format!(
                "{} --> {}\n",
                format_timestamp_vtt(segment.start_time_ms as i64),
                format_timestamp_vtt(segment.end_time_ms as i64)
            ));

            if request.include_speaker_names {
                if let Some(ref speaker) = segment.speaker_name {
                    output.push_str(&format!("<v {}>{}</v>\n\n", speaker, segment.text));
                    continue;
                }
            }
            output.push_str(&format!("{}\n\n", segment.text));
        }

        output
    }
}
