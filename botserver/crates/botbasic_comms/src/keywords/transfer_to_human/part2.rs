use super::*;

pub async fn execute_transfer(
    state: Arc<dyn BasicRuntime>,
    session: &UserSession,
    request: TransferToHumanRequest,
) -> TransferResult {
    let work_path = "./work";
    let bot_id = session.bot_id;

    if !is_crm_enabled(bot_id, work_path) {
        return TransferResult {
            success: false,
            status: TransferStatus::CrmDisabled,
            queue_position: None,
            assigned_to: None,
            assigned_to_name: None,
            estimated_wait_seconds: None,
            message: "CRM features are not enabled. Set 'crm-enabled' to 'true' in the bot configuration database"
                .to_string(),
        };
    }

    let attendants = read_attendants(bot_id, work_path);
    if attendants.is_empty() {
        return TransferResult {
            success: false,
            status: TransferStatus::NoAttendants,
            queue_position: None,
            assigned_to: None,
            assigned_to_name: None,
            estimated_wait_seconds: None,
            message: "No attendants configured. Create attendant.csv in your .gbai folder"
                .to_string(),
        };
    }

    let attendant = find_attendant(
        &attendants,
        request.name.as_deref(),
        request.department.as_deref(),
    );

    if request.name.is_some() && attendant.is_none() {
        return TransferResult {
            success: false,
            status: TransferStatus::AttendantNotFound,
            queue_position: None,
            assigned_to: None,
            assigned_to_name: None,
            estimated_wait_seconds: None,
            message: format!(
                "Attendant '{}' not found. Available attendants: {}",
                request.name.as_deref().unwrap_or("unknown"),
                attendants
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
    }

    let priority = priority_to_int(request.priority.as_deref());
    let transfer_context = serde_json::json!({
        "transfer_requested_at": Utc::now().to_rfc3339(),
        "transfer_priority": priority,
        "transfer_reason": request.reason.clone().unwrap_or_default(),
        "transfer_context": request.context.clone().unwrap_or_default(),
        "transfer_to_name": request.name.clone(),
        "transfer_to_department": request.department.clone(),
        "needs_human": true,
        "assigned_to": attendant.as_ref().map(|a| a.id.clone()),
        "assigned_to_name": attendant.as_ref().map(|a| a.name.clone()),
        "status": if attendant.is_some() { "assigned" } else { "queued" },
    });

    let session_id = session.id;
    let conn = state.db_pool().clone();
    let ctx_data = transfer_context.clone();

    let update_result = tokio::task::spawn_blocking(move || {
        let mut db_conn = conn
            .get()
            .map_err(|e| format!("DB connection error: {}", e))?;

        use botcore::shared::models::schema::user_sessions;

        diesel::update(user_sessions::table.filter(user_sessions::id.eq(session_id)))
            .set(user_sessions::context_data.eq(ctx_data))
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to update session: {}", e))
    })
    .await;

    match update_result {
        Ok(Ok(_)) => {
            if let Some(att) = attendant {
                info!(
                    "Transfer: Session {} assigned to {} ({})",
                    session.id, att.name, att.id
                );
                TransferResult {
                    success: true,
                    status: TransferStatus::Assigned,
                    queue_position: Some(1),
                    assigned_to: Some(att.id.clone()),
                    assigned_to_name: Some(att.name.clone()),
                    estimated_wait_seconds: Some(30),
                    message: format!(
                        "You have been connected to {}. They will be with you shortly.",
                        att.name
                    ),
                }
            } else {
                info!(
                    "Transfer: Session {} queued for next available attendant",
                    session.id
                );

                let queue_position = calculate_queue_position(&state, session_id).await;
                let estimated_wait = queue_position * 60;

                TransferResult {
                    success: true,
                    status: TransferStatus::Queued,
                    queue_position: Some(queue_position),
                    assigned_to: None,
                    assigned_to_name: None,
                    estimated_wait_seconds: Some(estimated_wait),
                    message: format!(
                        "You have been added to the queue at position {}. Estimated wait time: {} minutes.",
                        queue_position,
                        estimated_wait / 60
                    ),
                }
            }
        }
        Ok(Err(e)) => {
            log::error!("Transfer failed: {}", e);
            TransferResult {
                success: false,
                status: TransferStatus::Error,
                queue_position: None,
                assigned_to: None,
                assigned_to_name: None,
                estimated_wait_seconds: None,
                message: format!("Transfer failed: {}", e),
            }
        }
        Err(e) => {
            log::error!("Transfer task failed: {:?}", e);
            TransferResult {
                success: false,
                status: TransferStatus::Error,
                queue_position: None,
                assigned_to: None,
                assigned_to_name: None,
                estimated_wait_seconds: None,
                message: format!("Transfer task failed: {:?}", e),
            }
        }
    }
}
impl TransferResult {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();
        map.insert("success".into(), Dynamic::from(self.success));
        map.insert(
            "status".into(),
            Dynamic::from(format!("{:?}", self.status).to_lowercase()),
        );
        map.insert("message".into(), Dynamic::from(self.message.clone()));

        if let Some(pos) = self.queue_position {
            map.insert("queue_position".into(), Dynamic::from(pos));
        }
        if let Some(ref id) = self.assigned_to {
            map.insert("assigned_to".into(), Dynamic::from(id.clone()));
        }
        if let Some(ref name) = self.assigned_to_name {
            map.insert("assigned_to_name".into(), Dynamic::from(name.clone()));
        }
        if let Some(wait) = self.estimated_wait_seconds {
            map.insert("estimated_wait_seconds".into(), Dynamic::from(wait));
        }

        Dynamic::from(map)
    }
}
pub(super) async fn calculate_queue_position(state: &Arc<dyn BasicRuntime>, current_session_id: Uuid) -> i32 {
    let conn = state.db_pool().clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error calculating queue position: {}", e);
                return 1;
            }
        };

        let query = diesel::sql_query(
            r"SELECT COUNT(*) as position FROM user_sessions
               WHERE context_data->>'needs_human' = 'true'
               AND context_data->>'status' = 'queued'
               AND created_at <= (SELECT created_at FROM user_sessions WHERE id = $1)
               AND id != $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(current_session_id)
        .bind::<diesel::sql_types::Uuid, _>(current_session_id);

        #[derive(QueryableByName)]
        struct QueueCount {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            position: i64,
        }

        match query.get_result::<QueueCount>(&mut *db_conn) {
            Ok(count) => (count.position + 1) as i32,
            Err(e) => {
                debug!("Could not calculate queue position: {}", e);
                1
            }
        }
    })
    .await;

    result.unwrap_or(1)
}
