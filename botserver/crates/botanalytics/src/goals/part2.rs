use super::*;

pub async fn update_key_result(
    State(state): State<(Arc<DbPool>, GetBotContextFn)>,
    Path(key_result_id): Path<Uuid>,
    Json(req): Json<UpdateKeyResultRequest>,
) -> Result<Json<KeyResult>, GoalsError> {
    let (pool, _) = state;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| GoalsError::Database(e.to_string()))?;

        let mut kr = okr_key_results::table
            .find(key_result_id)
            .first::<KeyResultRecord>(&mut conn)
            .optional()
            .map_err(|e| GoalsError::Database(e.to_string()))?
            .ok_or_else(|| GoalsError::NotFound("Key result not found".to_string()))?;

        if let Some(title) = req.title {
            kr.title = title;
        }
        if let Some(description) = req.description {
            kr.description = Some(description);
        }
        if let Some(target_value) = req.target_value {
            kr.target_value = BigDecimal::try_from(target_value).unwrap_or_else(|_| BigDecimal::from(0));
        }
        if let Some(current_value) = req.current_value {
            kr.current_value = BigDecimal::try_from(current_value).unwrap_or_else(|_| BigDecimal::from(0));
        }
        if let Some(weight) = req.weight {
            kr.weight = BigDecimal::try_from(weight as f64).unwrap_or_else(|_| BigDecimal::from(1));
        }
        if let Some(due_date) = req.due_date {
            kr.due_date = Some(due_date);
        }
        if let Some(status) = req.status {
            kr.status = status.to_str().to_string();
        }
        kr.updated_at = Utc::now();

        diesel::update(okr_key_results::table.find(key_result_id))
            .set(&kr)
            .execute(&mut conn)
            .map_err(|e| GoalsError::Database(e.to_string()))?;

        Ok::<_, GoalsError>(kr)
    })
    .await
    .map_err(|e| GoalsError::Database(e.to_string()))??;

    info!("Updated key result: {} ({})", result.title, result.id);
    Ok(Json(record_to_key_result(result)))
}
pub async fn delete_key_result(
    State(state): State<(Arc<DbPool>, GetBotContextFn)>,
    Path(key_result_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, GoalsError> {
    let (pool, _) = state;

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| GoalsError::Database(e.to_string()))?;
        let deleted = diesel::delete(okr_key_results::table.find(key_result_id))
            .execute(&mut conn)
            .map_err(|e| GoalsError::Database(e.to_string()))?;

        if deleted > 0 {
            info!("Deleted key result: {key_result_id}");
            Ok::<_, GoalsError>(())
        } else {
            Err(GoalsError::NotFound("Key result not found".to_string()))
        }
    })
    .await
    .map_err(|e| GoalsError::Database(e.to_string()))??;

    Ok(Json(serde_json::json!({ "success": true })))
}
pub async fn create_check_in(
    State(state): State<(Arc<DbPool>, GetBotContextFn)>,
    user: AuthenticatedUser,
    Path(key_result_id): Path<Uuid>,
    Json(req): Json<CreateCheckInRequest>,
) -> Result<Json<CheckIn>, GoalsError> {
    let (pool, get_bot_context) = state;
    let (org_id, branch_id) = get_bot_context();
    let user_id = user.user_id;
    let now = Utc::now();

    let pool_clone = pool.clone();
    let previous_value = tokio::task::spawn_blocking(move || {
        let mut conn = pool_clone.get().ok()?;
        okr_key_results::table
            .find(key_result_id)
            .select(okr_key_results::current_value)
            .first::<BigDecimal>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    let new_checkin = CheckInRecord {
        id: Uuid::new_v4(),
        org_id,
        branch_id,
        key_result_id,
        user_id,
        previous_value,
        new_value: BigDecimal::try_from(req.new_value).unwrap_or_else(|_| BigDecimal::from(0)),
        note: req.note.clone(),
        confidence: req.confidence.as_ref().map(|c| c.to_str().to_string()),
        blockers: req.blockers.clone(),
        created_at: now,
    };

    let record = new_checkin.clone();
    let new_val = req.new_value;

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| GoalsError::Database(e.to_string()))?;

        diesel::insert_into(okr_checkins::table)
            .values(&new_checkin)
            .execute(&mut conn)
            .map_err(|e| GoalsError::Database(e.to_string()))?;

        diesel::update(okr_key_results::table.find(key_result_id))
            .set((
                okr_key_results::current_value.eq(BigDecimal::try_from(new_val).unwrap_or_else(|_| BigDecimal::from(0))),
                okr_key_results::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| GoalsError::Database(e.to_string()))?;

        Ok::<_, GoalsError>(())
    })
    .await
    .map_err(|e| GoalsError::Database(e.to_string()))??;

    info!("Created check-in for key result: {key_result_id}");
    Ok(Json(record_to_checkin(record)))
}
pub async fn get_check_in_history(
    State(state): State<(Arc<DbPool>, GetBotContextFn)>,
    Path(key_result_id): Path<Uuid>,
) -> Result<Json<Vec<CheckIn>>, GoalsError> {
    let (pool, _) = state;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| GoalsError::Database(e.to_string()))?;
        okr_checkins::table
            .filter(okr_checkins::key_result_id.eq(key_result_id))
            .order(okr_checkins::created_at.desc())
            .load::<CheckInRecord>(&mut conn)
            .map_err(|e| GoalsError::Database(e.to_string()))
    })
    .await
    .map_err(|e| GoalsError::Database(e.to_string()))??;

    let history: Vec<CheckIn> = result.into_iter().map(record_to_checkin).collect();
    Ok(Json(history))
}
pub async fn get_dashboard(
    State(state): State<(Arc<DbPool>, GetBotContextFn)>,
) -> Result<Json<GoalsDashboard>, GoalsError> {
    let (pool, get_bot_context) = state;
    let (org_id, branch_id) = get_bot_context();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| GoalsError::Database(e.to_string()))?;

        let total: i64 = okr_objectives::table
            .filter(okr_objectives::org_id.eq(org_id))
            .filter(okr_objectives::branch_id.eq(branch_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let completed: i64 = okr_objectives::table
            .filter(okr_objectives::org_id.eq(org_id))
            .filter(okr_objectives::branch_id.eq(branch_id))
            .filter(okr_objectives::status.eq("completed"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let at_risk: i64 = okr_objectives::table
            .filter(okr_objectives::org_id.eq(org_id))
            .filter(okr_objectives::branch_id.eq(branch_id))
            .filter(okr_objectives::status.eq("at_risk"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let total_key_results: i64 = okr_key_results::table
            .filter(okr_key_results::org_id.eq(org_id))
            .filter(okr_key_results::branch_id.eq(branch_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let at_risk_key_results: i64 = okr_key_results::table
            .filter(okr_key_results::org_id.eq(org_id))
            .filter(okr_key_results::branch_id.eq(branch_id))
            .filter(okr_key_results::status.eq("at_risk"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let objectives = okr_objectives::table
            .filter(okr_objectives::org_id.eq(org_id))
            .filter(okr_objectives::branch_id.eq(branch_id))
            .select(okr_objectives::progress)
            .load::<BigDecimal>(&mut conn)
            .unwrap_or_default();

        let avg_progress = if objectives.is_empty() {
            0.0
        } else {
            let sum: f32 = objectives.iter().map(|p| p.to_f32().unwrap_or(0.0)).sum();
            sum / objectives.len() as f32
        };

        let upcoming_krs = okr_key_results::table
            .filter(okr_key_results::org_id.eq(org_id))
            .filter(okr_key_results::branch_id.eq(branch_id))
            .filter(okr_key_results::due_date.is_not_null())
            .order(okr_key_results::due_date.asc())
            .limit(5)
            .load::<KeyResultRecord>(&mut conn)
            .unwrap_or_default();

        let upcoming_check_ins: Vec<UpcomingCheckIn> = upcoming_krs.into_iter().map(|kr| {
            UpcomingCheckIn {
                key_result_id: kr.id,
                key_result_title: kr.title,
                objective_title: String::new(),
                due_date: kr.due_date,
            }
        }).collect();

        Ok::<_, GoalsError>(GoalsDashboard {
            total_objectives: total,
            completed_objectives: completed,
            at_risk_objectives: at_risk,
            total_key_results,
            at_risk_key_results,
            average_progress: avg_progress,
            upcoming_check_ins,
            recent_activity: vec![],
        })
    })
    .await
    .map_err(|e| GoalsError::Database(e.to_string()))??;

    Ok(Json(result))
}
pub async fn get_alignment(
    State(state): State<(Arc<DbPool>, GetBotContextFn)>,
) -> Result<Json<Vec<AlignmentNode>>, GoalsError> {
    let (pool, get_bot_context) = state;
    let (org_id, branch_id) = get_bot_context();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| GoalsError::Database(e.to_string()))?;

        let objectives = okr_objectives::table
            .filter(okr_objectives::org_id.eq(org_id))
            .filter(okr_objectives::branch_id.eq(branch_id))
            .filter(okr_objectives::parent_id.is_null())
            .load::<ObjectiveRecord>(&mut conn)
            .map_err(|e| GoalsError::Database(e.to_string()))?;

        let nodes: Vec<AlignmentNode> = objectives.into_iter().map(|obj| {
            let key_results = okr_key_results::table
                .filter(okr_key_results::objective_id.eq(obj.id))
                .load::<KeyResultRecord>(&mut conn)
                .unwrap_or_default()
                .into_iter()
                .map(record_to_key_result)
                .collect();

            AlignmentNode {
                objective: record_to_objective(obj),
                key_results,
                children: vec![],
            }
        }).collect();

        Ok::<_, GoalsError>(nodes)
    })
    .await
    .map_err(|e| GoalsError::Database(e.to_string()))??;

    Ok(Json(result))
}
