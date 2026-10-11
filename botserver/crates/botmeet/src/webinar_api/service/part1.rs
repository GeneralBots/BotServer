use super::*;

impl WebinarService {
    pub fn new(
        pool: Arc<diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
    ) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        Self { pool, event_sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<crate::webinar_api::types::WebinarEvent> {
        self.event_sender.subscribe()
    }

    pub async fn create_webinar(
        &self,
        organization_id: Uuid,
        host_id: Uuid,
        request: CreateWebinarRequest,
    ) -> Result<Webinar, WebinarError> {
        let mut conn = self.pool.get().map_err(|e| {
            log::error!("Failed to get database connection: {e}");
            WebinarError::DatabaseConnection
        })?;

        let id = Uuid::new_v4();
        let meeting_id = Uuid::new_v4();
        let settings = request.settings.unwrap_or_default();
        let settings_json = serde_json::to_string(&settings).unwrap_or_else(|_| "{}".to_string());

        let registration_url = if request.registration_required {
            Some(format!("/webinar/{}/register", id))
        } else {
            None
        };

        let sql = r#"
            INSERT INTO webinars (
                id, organization_id, meeting_id, title, description,
                scheduled_start, scheduled_end, status, settings_json,
                registration_required, registration_url, host_id,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, 'scheduled', $8, $9, $10, $11, NOW(), NOW()
            )
        "#;

        diesel::sql_query(sql)
            .bind::<DieselUuid, _>(id)
            .bind::<DieselUuid, _>(organization_id)
            .bind::<DieselUuid, _>(meeting_id)
            .bind::<Text, _>(&request.title)
            .bind::<Nullable<Text>, _>(request.description.as_deref())
            .bind::<Timestamptz, _>(request.scheduled_start)
            .bind::<Nullable<Timestamptz>, _>(request.scheduled_end)
            .bind::<Text, _>(&settings_json)
            .bind::<Bool, _>(request.registration_required)
            .bind::<Nullable<Text>, _>(registration_url.as_deref())
            .bind::<DieselUuid, _>(host_id)
            .execute(&mut conn)
            .map_err(|e| {
                log::error!("Failed to create webinar: {e}");
                WebinarError::CreateFailed
            })?;

        self.add_participant_internal(
            &mut conn,
            id,
            Some(host_id),
            "Host".to_string(),
            None,
            ParticipantRole::Host,
        )?;

        if let Some(panelists) = request.panelists {
            for panelist in panelists {
                self.add_participant_internal(
                    &mut conn,
                    id,
                    None,
                    panelist.name,
                    Some(panelist.email),
                    panelist.role,
                )?;
            }
        }

        info!("Created webinar {} for org {}", id, organization_id);

        self.get_webinar(id).await
    }

    pub async fn get_webinar(&self, webinar_id: Uuid) -> Result<Webinar, WebinarError> {
        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, organization_id, meeting_id, title, description,
                   scheduled_start, scheduled_end, actual_start, actual_end,
                   status, settings_json, registration_required, registration_url,
                   host_id, created_at, updated_at
            FROM webinars WHERE id = $1
        "#;

        let rows: Vec<WebinarRow> = diesel::sql_query(sql)
            .bind::<DieselUuid, _>(webinar_id)
            .load(&mut conn)
            .map_err(|e| {
                log::error!("Failed to get webinar: {e}");
                WebinarError::DatabaseConnection
            })?;

        let row = rows.into_iter().next().ok_or(WebinarError::NotFound)?;
        Ok(self.row_to_webinar(row))
    }

    pub async fn start_webinar(&self, webinar_id: Uuid, host_id: Uuid) -> Result<Webinar, WebinarError> {
        let webinar = self.get_webinar(webinar_id).await?;

        if webinar.host_id != host_id {
            return Err(WebinarError::NotAuthorized);
        }

        if webinar.status != WebinarStatus::Scheduled && webinar.status != WebinarStatus::Paused {
            return Err(WebinarError::InvalidState("Webinar cannot be started".to_string()));
        }

        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        diesel::sql_query(
            "UPDATE webinars SET status = 'live', actual_start = COALESCE(actual_start, NOW()), updated_at = NOW() WHERE id = $1"
        )
        .bind::<DieselUuid, _>(webinar_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to start webinar: {e}");
            WebinarError::UpdateFailed
        })?;

        self.broadcast_event(WebinarEventType::WebinarStarted, webinar_id, serde_json::json!({}));

        info!("Started webinar {}", webinar_id);
        self.get_webinar(webinar_id).await
    }

    pub async fn end_webinar(&self, webinar_id: Uuid, host_id: Uuid) -> Result<Webinar, WebinarError> {
        let webinar = self.get_webinar(webinar_id).await?;

        if webinar.host_id != host_id {
            return Err(WebinarError::NotAuthorized);
        }

        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        diesel::sql_query(
            "UPDATE webinars SET status = 'ended', actual_end = NOW(), updated_at = NOW() WHERE id = $1"
        )
        .bind::<DieselUuid, _>(webinar_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to end webinar: {e}");
            WebinarError::UpdateFailed
        })?;

        self.broadcast_event(WebinarEventType::WebinarEnded, webinar_id, serde_json::json!({}));

        info!("Ended webinar {}", webinar_id);
        self.get_webinar(webinar_id).await
    }

    pub async fn register_attendee(
        &self,
        webinar_id: Uuid,
        request: RegisterRequest,
    ) -> Result<WebinarRegistration, WebinarError> {
        let webinar = self.get_webinar(webinar_id).await?;

        if !webinar.registration_required {
            return Err(WebinarError::RegistrationNotRequired);
        }

        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let existing: Vec<CountRow> = diesel::sql_query(
            "SELECT COUNT(*) as count FROM webinar_registrations WHERE webinar_id = $1 AND email = $2"
        )
        .bind::<DieselUuid, _>(webinar_id)
        .bind::<Text, _>(&request.email)
        .load(&mut conn)
        .unwrap_or_default();

        if existing.first().map(|r| r.count > 0).unwrap_or(false) {
            return Err(WebinarError::AlreadyRegistered);
        }

        let id = Uuid::new_v4();
        let join_link = format!("/webinar/{}/join?token={}", webinar_id, Uuid::new_v4());
        let custom_fields = request.custom_fields.clone().unwrap_or_default();
        let custom_fields_json = serde_json::to_string(&custom_fields)
            .unwrap_or_else(|_| "{}".to_string());

        let sql = r#"
            INSERT INTO webinar_registrations (
                id, webinar_id, email, name, custom_fields, status, join_link,
                registered_at, confirmed_at
            ) VALUES ($1, $2, $3, $4, $5, 'confirmed', $6, NOW(), NOW())
        "#;

        diesel::sql_query(sql)
            .bind::<DieselUuid, _>(id)
            .bind::<DieselUuid, _>(webinar_id)
            .bind::<Text, _>(&request.email)
            .bind::<Text, _>(&request.name)
            .bind::<Text, _>(&custom_fields_json)
            .bind::<Text, _>(&join_link)
            .execute(&mut conn)
            .map_err(|e| {
                log::error!("Failed to register: {e}");
                WebinarError::RegistrationFailed
            })?;

        self.add_participant_internal(
            &mut conn,
            webinar_id,
            None,
            request.name.clone(),
            Some(request.email.clone()),
            ParticipantRole::Attendee,
        )?;

        Ok(WebinarRegistration {
            id,
            webinar_id,
            email: request.email,
            name: request.name,
            custom_fields,
            status: RegistrationStatus::Confirmed,
            join_link,
            registered_at: Utc::now(),
            confirmed_at: Some(Utc::now()),
            cancelled_at: None,
        })
    }

    pub async fn join_webinar(
        &self,
        webinar_id: Uuid,
        participant_id: Uuid,
    ) -> Result<WebinarParticipant, WebinarError> {
        let webinar = self.get_webinar(webinar_id).await?;

        if webinar.status != WebinarStatus::Live && webinar.status != WebinarStatus::Scheduled {
            return Err(WebinarError::InvalidState("Webinar is not active".to_string()));
        }

        let mut conn = self.pool.get().map_err(|_| WebinarError::DatabaseConnection)?;

        let status = if webinar.settings.waiting_room_enabled {
            "in_waiting_room"
        } else {
            "joined"
        };

        diesel::sql_query(
            "UPDATE webinar_participants SET status = $1, joined_at = NOW() WHERE id = $2"
        )
        .bind::<Text, _>(status)
        .bind::<DieselUuid, _>(participant_id)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to join webinar: {e}");
            WebinarError::JoinFailed
        })?;

        self.broadcast_event(
            WebinarEventType::ParticipantJoined,
            webinar_id,
            serde_json::json!({"participant_id": participant_id}),
        );

        self.get_participant(participant_id).await
    }
}
