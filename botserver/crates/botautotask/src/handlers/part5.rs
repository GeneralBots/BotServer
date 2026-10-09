use super::*;

impl AutoTaskRow {
    pub(super) fn into_json(self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "bot_id": self.bot_id,
            "title": self.title,
            "intent": self.intent,
            "status": self.status,
            "script_path": self.basic_program,
            "created_at": self.created_at.to_rfc3339(),
            "updated_at": self.updated_at.to_rfc3339(),
        })
    }
}
pub async fn list_tasks(
    State(api): State<Arc<AutoTaskApi>>,
    Query(query): Query<crate::api::ListTasksQuery>,
) -> Json<Vec<serde_json::Value>> {
    let pool = api.state().db_pool().clone();
    let bot_filter = query.bot_id.clone().and_then(|s| Uuid::parse_str(&s).ok());
    let rows = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        match bot_filter {
            Some(bot_id) => sql_query(
                "SELECT id, bot_id, title, intent, status, basic_program, created_at, updated_at \
                 FROM auto_tasks WHERE bot_id = $1 ORDER BY created_at DESC LIMIT 100",
            )
            .bind::<DieselUuid, _>(bot_id)
            .load::<AutoTaskRow>(&mut conn)
            .ok(),
            None => sql_query(
                "SELECT id, bot_id, title, intent, status, basic_program, created_at, updated_at \
                 FROM auto_tasks ORDER BY created_at DESC LIMIT 100",
            )
            .load::<AutoTaskRow>(&mut conn)
            .ok(),
        }
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_default();
    Json(rows.into_iter().map(|r| r.into_json()).collect())
}
pub async fn get_stats(
    State(api): State<Arc<AutoTaskApi>>,
) -> impl IntoResponse {
    #[derive(diesel::QueryableByName)]
    struct CountRow {
        #[diesel(sql_type = BigInt)]
        total: i64,
        #[diesel(sql_type = BigInt)]
        running: i64,
        #[diesel(sql_type = BigInt)]
        pending: i64,
        #[diesel(sql_type = BigInt)]
        completed: i64,
        #[diesel(sql_type = BigInt)]
        failed: i64,
        #[diesel(sql_type = BigInt)]
        pending_approval: i64,
    }
    let mut stats = crate::api::AutoTaskStatsResponse {
        total: 0, running: 0, pending: 0, completed: 0, failed: 0, pending_approval: 0, pending_decision: 0,
    };
    if let Ok(mut conn) = api.state().db_pool().get() {
        // #1505 — the item list is auto_tasks, so the stats must count the same
        // rows (intent_classifications counted heuristic runs, not items).
        if let Ok(row) = sql_query(
            "SELECT COUNT(*) AS total, \
                    COUNT(*) FILTER (WHERE status = 'running') AS running, \
                    COUNT(*) FILTER (WHERE status IN ('pending','ready','paused')) AS pending, \
                    COUNT(*) FILTER (WHERE status = 'completed') AS completed, \
                    COUNT(*) FILTER (WHERE status = 'failed') AS failed, \
                    COUNT(*) FILTER (WHERE status = 'waiting_approval') AS pending_approval \
             FROM auto_tasks",
        )
        .get_result::<CountRow>(&mut conn)
        {
            stats.total = row.total as i32;
            stats.running = row.running as i32;
            stats.pending = row.pending as i32;
            stats.completed = row.completed as i32;
            stats.failed = row.failed as i32;
            stats.pending_approval = row.pending_approval as i32;
        }
    }
    Json(stats)
}
pub async fn approve_task(
    State(api): State<Arc<AutoTaskApi>>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    info!("API approve task: {task_id}");
    let task_uuid = match uuid::Uuid::parse_str(&task_id) {
        Ok(u) => u,
        Err(_) => return Json(TaskActionResponse { success: false, message: None, error: Some("Invalid task id".to_string()) }),
    };
    let pool = api.state().db_pool().clone();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut conn = pool.get().map_err(|e| format!("DB connection failed: {e}"))?;
        diesel::sql_query(
            "UPDATE auto_tasks SET status = 'ready', updated_at = NOW() WHERE id = $1 AND status IN ('pending', 'waiting_approval')",
        )
        .bind::<diesel::sql_types::Uuid, _>(task_uuid)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to approve task: {e}"))?;
        diesel::sql_query(
            "UPDATE task_approvals SET status = 'approved', decision = 'approve', decided_at = NOW() WHERE task_id = $1 AND status = 'pending'",
        )
        .bind::<diesel::sql_types::Uuid, _>(task_uuid)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to approve task approvals: {e}"))?;
        Ok(())
    })
    .await;
    match result {
        Ok(Ok(())) => Json(TaskActionResponse { success: true, message: Some("Task approved".to_string()), error: None }),
        Ok(Err(e)) => {
            log::error!("Failed to approve task {task_id}: {e}");
            Json(TaskActionResponse { success: false, message: None, error: Some(e) })
        }
        Err(e) => {
            log::error!("Approve task task panicked: {e}");
            Json(TaskActionResponse { success: false, message: None, error: Some("Task execution failed".to_string()) })
        }
    }
}
pub async fn cancel_task(
    State(api): State<Arc<AutoTaskApi>>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    info!("API cancel task: {task_id}");
    let task_uuid = match uuid::Uuid::parse_str(&task_id) {
        Ok(u) => u,
        Err(_) => return Json(TaskActionResponse { success: false, message: None, error: Some("Invalid task id".to_string()) }),
    };
    let pool = api.state().db_pool().clone();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut conn = pool.get().map_err(|e| format!("DB connection failed: {e}"))?;
        diesel::sql_query(
            "UPDATE auto_tasks SET status = 'cancelled', updated_at = NOW() WHERE id = $1 AND status NOT IN ('completed', 'cancelled')",
        )
        .bind::<diesel::sql_types::Uuid, _>(task_uuid)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to cancel task: {e}"))?;
        Ok(())
    })
    .await;
    match result {
        Ok(Ok(())) => Json(TaskActionResponse { success: true, message: Some("Task cancelled".to_string()), error: None }),
        Ok(Err(e)) => {
            log::error!("Failed to cancel task {task_id}: {e}");
            Json(TaskActionResponse { success: false, message: None, error: Some(e) })
        }
        Err(e) => {
            log::error!("Cancel task task panicked: {e}");
            Json(TaskActionResponse { success: false, message: None, error: Some("Task execution failed".to_string()) })
        }
    }
}
pub async fn make_decision(
    State(api): State<Arc<AutoTaskApi>>,
    Json(req): Json<DecisionRequest>,
) -> impl IntoResponse {
    info!("API make decision: {} -> {}", req.decision_id, req.choice);
    let decision_uuid = match uuid::Uuid::parse_str(&req.decision_id) {
        Ok(u) => u,
        Err(_) => return Json(TaskActionResponse { success: false, message: None, error: Some("Invalid decision id".to_string()) }),
    };
    let choice = req.choice.clone();
    let pool = api.state().db_pool().clone();
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut conn = pool.get().map_err(|e| format!("DB connection failed: {e}"))?;
        diesel::sql_query(
            "UPDATE task_decisions SET status = 'answered', selected_option = $1, decided_at = NOW() WHERE id = $2 AND status = 'pending'",
        )
        .bind::<diesel::sql_types::Text, _>(&choice)
        .bind::<diesel::sql_types::Uuid, _>(decision_uuid)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to record decision: {e}"))?;
        Ok(())
    })
    .await;
    match result {
        Ok(Ok(())) => Json(TaskActionResponse { success: true, message: Some("Decision recorded".to_string()), error: None }),
        Ok(Err(e)) => {
            log::error!("Failed to record decision {}: {e}", req.decision_id);
            Json(TaskActionResponse { success: false, message: None, error: Some(e) })
        }
        Err(e) => {
            log::error!("Decision task panicked: {e}");
            Json(TaskActionResponse { success: false, message: None, error: Some("Decision execution failed".to_string()) })
        }
    }
}
