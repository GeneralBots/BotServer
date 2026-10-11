use super::*;

impl WebinarService {
    pub async fn raise_hand(&self, webinar_id: Uuid, participant_id: Uuid) -> Result<(), WebinarError> {
        let webinar = self.get_webinar(webinar_id).await?;

        if !webinar.settings.allow_hand_raise {
            return Err(WebinarError::FeatureDisabled("Hand raising is disabled".to_string()));
        }

        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        diesel::sql_query(
            "UPDATE webinar_participants SET hand_raised = TRUE, hand_raised_at = NOW() WHERE id = $1 AND webinar_id = $2"
        )
        .bind::<DieselUuid, _>(participant_id)
        .bind::<DieselUuid, _>(webinar_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to raise hand: {e}");
            WebinarError::UpdateFailed
        })?;

        self.broadcast_event(
            WebinarEventType::HandRaised,
            webinar_id,
            serde_json::json!({"participant_id": participant_id}),
        );

        Ok(())
    }

    pub async fn lower_hand(&self, webinar_id: Uuid, participant_id: Uuid) -> Result<(), WebinarError> {
        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        diesel::sql_query(
            "UPDATE webinar_participants SET hand_raised = FALSE, hand_raised_at = NULL WHERE id = $1 AND webinar_id = $2"
        )
        .bind::<DieselUuid, _>(participant_id)
        .bind::<DieselUuid, _>(webinar_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to lower hand: {e}");
            WebinarError::UpdateFailed
        })?;

        self.broadcast_event(
            WebinarEventType::HandLowered,
            webinar_id,
            serde_json::json!({"participant_id": participant_id}),
        );

        Ok(())
    }

    pub async fn get_raised_hands(&self, webinar_id: Uuid) -> Result<Vec<WebinarParticipant>, WebinarError> {
        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, webinar_id, user_id, name, email, role, status,
                   hand_raised, hand_raised_at, is_speaking, video_enabled,
                   audio_enabled, screen_sharing, joined_at, left_at, registration_data
            FROM webinar_participants
            WHERE webinar_id = $1 AND hand_raised = TRUE
            ORDER BY hand_raised_at ASC
            LIMIT $2
        "#;

        let rows: Vec<ParticipantRow> = diesel::sql_query(sql)
            .bind::<DieselUuid, _>(webinar_id)
            .bind::<Integer, _>(MAX_RAISED_HANDS_VISIBLE as i32)
            .load(&mut conn)
            .unwrap_or_default();

        Ok(rows.into_iter().map(|r| self.row_to_participant(r)).collect())
    }

    pub async fn submit_question(
        &self,
        webinar_id: Uuid,
        asker_id: Option<Uuid>,
        asker_name: String,
        request: crate::webinar_api::types::SubmitQuestionRequest,
    ) -> Result<QAQuestion, WebinarError> {
        let webinar = self.get_webinar(webinar_id).await?;

        if !webinar.settings.allow_qa {
            return Err(WebinarError::FeatureDisabled("Q&A is disabled".to_string()));
        }

        if request.question.len() > QA_QUESTION_MAX_LENGTH {
            return Err(WebinarError::InvalidInput("Question too long".to_string()));
        }

        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let id = Uuid::new_v4();
        let is_anonymous = request.is_anonymous.unwrap_or(false) && webinar.settings.anonymous_qa;
        let status = if webinar.settings.moderated_qa { "pending" } else { "approved" };
        let display_name = if is_anonymous { "Anonymous".to_string() } else { asker_name };

        let sql = r#"
            INSERT INTO webinar_questions (
                id, webinar_id, asker_id, asker_name, is_anonymous, question,
                status, upvotes, is_pinned, is_highlighted, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, 0, FALSE, FALSE, NOW())
        "#;

        diesel::sql_query(sql)
            .bind::<DieselUuid, _>(id)
            .bind::<DieselUuid, _>(webinar_id)
            .bind::<Nullable<DieselUuid>, _>(asker_id)
            .bind::<Text, _>(&display_name)
            .bind::<Bool, _>(is_anonymous)
            .bind::<Text, _>(&request.question)
            .bind::<Text, _>(status)
            .execute(&mut conn)
            .map_err(|e| {
                log::error!("Failed to submit question: {e}");
                WebinarError::CreateFailed
            })?;

        self.broadcast_event(
            WebinarEventType::QuestionSubmitted,
            webinar_id,
            serde_json::json!({"question_id": id}),
        );

        Ok(QAQuestion {
            id,
            webinar_id,
            asker_id,
            asker_name: display_name,
            is_anonymous,
            question: request.question,
            status: if webinar.settings.moderated_qa { QuestionStatus::Pending } else { QuestionStatus::Approved },
            upvotes: 0,
            upvoted_by: vec![],
            answer: None,
            answered_by: None,
            answered_at: None,
            is_pinned: false,
            is_highlighted: false,
            created_at: Utc::now(),
        })
    }

    pub async fn answer_question(
        &self,
        question_id: Uuid,
        answerer_id: Uuid,
        request: crate::webinar_api::types::AnswerQuestionRequest,
    ) -> Result<QAQuestion, WebinarError> {
        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let status = if request.mark_as_live.unwrap_or(false) { "answered_live" } else { "answered" };

        diesel::sql_query(
            "UPDATE webinar_questions SET answer = $1, answered_by = $2, answered_at = NOW(), status = $3 WHERE id = $4"
        )
        .bind::<Text, _>(&request.answer)
        .bind::<DieselUuid, _>(answerer_id)
        .bind::<Text, _>(status)
        .bind::<DieselUuid, _>(question_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to answer question: {e}");
            WebinarError::UpdateFailed
        })?;

        self.get_question(question_id).await
    }

    pub async fn upvote_question(&self, question_id: Uuid, voter_id: Uuid) -> Result<QAQuestion, WebinarError> {
        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        diesel::sql_query(
            "UPDATE webinar_questions SET upvotes = upvotes + 1, upvoted_by = COALESCE(upvoted_by, '[]')::jsonb || $1::jsonb WHERE id = $2"
        )
        .bind::<Text, _>(serde_json::json!([voter_id]).to_string())
        .bind::<DieselUuid, _>(question_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to upvote question: {e}");
            WebinarError::UpdateFailed
        })?;

        self.get_question(question_id).await
    }

    pub async fn get_questions(&self, webinar_id: Uuid, include_pending: bool) -> Result<Vec<QAQuestion>, WebinarError> {
        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let status_filter = if include_pending { "" } else { "AND status != 'pending'" };

        let sql = format!(r#"
            SELECT id, webinar_id, asker_id, asker_name, is_anonymous, question,
                   status, upvotes, upvoted_by, answer, answered_by, answered_at,
                   is_pinned, is_highlighted, created_at
            FROM webinar_questions
            WHERE webinar_id = $1 {status_filter}
            ORDER BY is_pinned DESC, upvotes DESC, created_at ASC
        "#);

        let rows: Vec<QuestionRow> = diesel::sql_query(&sql)
            .bind::<DieselUuid, _>(webinar_id)
            .load(&mut conn)
            .unwrap_or_default();

        Ok(rows.into_iter().map(|r| self.row_to_question(r)).collect())
    }

    async fn get_question(&self, question_id: Uuid) -> Result<QAQuestion, WebinarError> {
        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, webinar_id, asker_id, asker_name, is_anonymous, question,
                   status, upvotes, upvoted_by, answer, answered_by, answered_at,
                   is_pinned, is_highlighted, created_at
            FROM webinar_questions WHERE id = $1
        "#;

        let rows: Vec<QuestionRow> = diesel::sql_query(sql)
            .bind::<DieselUuid, _>(question_id)
            .load(&mut conn)
            .map_err(|_| WebinarError::DatabaseConnection)?;

        let row = rows.into_iter().next().ok_or(WebinarError::NotFound)?;
        Ok(self.row_to_question(row))
    }

    pub(crate) async fn get_participant(&self, participant_id: Uuid) -> Result<WebinarParticipant, WebinarError> {
        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, webinar_id, user_id, name, email, role, status,
                   hand_raised, hand_raised_at, is_speaking, video_enabled,
                   audio_enabled, screen_sharing, joined_at, left_at, registration_data
            FROM webinar_participants WHERE id = $1
        "#;

        let rows: Vec<ParticipantRow> = diesel::sql_query(sql)
            .bind::<DieselUuid, _>(participant_id)
            .load(&mut conn)
            .map_err(|_| WebinarError::DatabaseConnection)?;

        let row = rows.into_iter().next().ok_or(WebinarError::NotFound)?;
        Ok(self.row_to_participant(row))
    }

    pub(super) fn add_participant_internal(
        &self,
        conn: &mut diesel::PgConnection,
        webinar_id: Uuid,
        user_id: Option<Uuid>,
        name: String,
        email: Option<String>,
        role: ParticipantRole,
    ) -> Result<Uuid, WebinarError> {
        let id = Uuid::new_v4();

        diesel::sql_query(r#"
            INSERT INTO webinar_participants (
                id, webinar_id, user_id, name, email, role, status,
                hand_raised, is_speaking, video_enabled, audio_enabled, screen_sharing
            ) VALUES ($1, $2, $3, $4, $5, $6, 'registered', FALSE, FALSE, FALSE, FALSE, FALSE)
        "#)
        .bind::<DieselUuid, _>(id)
        .bind::<DieselUuid, _>(webinar_id)
        .bind::<Nullable<DieselUuid>, _>(user_id)
        .bind::<Text, _>(&name)
        .bind::<Nullable<Text>, _>(email.as_deref())
        .bind::<Text, _>(role.to_string())
        .execute(conn)
        .map_err(|e| {
            log::error!("Failed to add participant: {e}");
            WebinarError::CreateFailed
        })?;

        Ok(id)
    }

    pub(super) fn broadcast_event(&self, event_type: WebinarEventType, webinar_id: Uuid, data: serde_json::Value) {
        let event = crate::webinar_api::types::WebinarEvent {
            event_type,
            webinar_id,
            data,
            timestamp: Utc::now(),
        };
        let _ = self.event_sender.send(event);
    }
}
