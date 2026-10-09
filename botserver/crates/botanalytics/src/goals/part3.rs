use super::*;

pub async fn ai_suggest(
    Json(_req): Json<AISuggestRequest>,
) -> Result<Json<Vec<AISuggestion>>, GoalsError> {
    let suggestions = vec![
        AISuggestion {
            objective: ObjectiveTemplate {
                title: "Improve customer satisfaction".to_string(),
                description: "Enhance customer experience across all touchpoints".to_string(),
            },
            key_results: vec![
                KeyResultTemplate {
                    title: "Increase NPS score".to_string(),
                    metric_type: MetricType::Number,
                    suggested_target: Some(50.0),
                },
                KeyResultTemplate {
                    title: "Reduce support ticket resolution time".to_string(),
                    metric_type: MetricType::Number,
                    suggested_target: Some(24.0),
                },
            ],
            rationale: "Customer satisfaction directly impacts retention and growth".to_string(),
        },
    ];
    Ok(Json(suggestions))
}
pub async fn list_templates(
    State(state): State<(Arc<DbPool>, GetBotContextFn)>,
) -> Result<Json<Vec<GoalTemplate>>, GoalsError> {
    let (pool, get_bot_context) = state;
    let (org_id, _bot_id) = get_bot_context();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| GoalsError::Database(e.to_string()))?;

        okr_templates::table
            .filter(okr_templates::org_id.eq(org_id).or(okr_templates::is_system.eq(true)))
            .order(okr_templates::name.asc())
            .load::<TemplateRecord>(&mut conn)
            .map_err(|e| GoalsError::Database(e.to_string()))
    })
    .await
    .map_err(|e| GoalsError::Database(e.to_string()))??;

    let templates: Vec<GoalTemplate> = result.into_iter().map(|t| {
        let objective_template: ObjectiveTemplate = serde_json::from_value(t.objective_template)
            .unwrap_or(ObjectiveTemplate { title: String::new(), description: String::new() });
        let key_result_templates: Vec<KeyResultTemplate> = serde_json::from_value(t.key_result_templates)
            .unwrap_or_default();

        GoalTemplate {
            id: t.id,
            organization_id: t.org_id,
            name: t.name,
            description: t.description,
            category: t.category,
            objective_template,
            key_result_templates,
            is_system: t.is_system,
            created_at: t.created_at,
        }
    }).collect();

    Ok(Json(templates))
}
pub fn configure_goals_routes() -> Router<(Arc<DbPool>, GetBotContextFn)> {
    Router::new()
        .route("/api/goals/objectives", get(list_objectives).post(create_objective))
        .route("/api/goals/objectives/:id", get(get_objective).put(update_objective).delete(delete_objective))
        .route("/api/goals/objectives/:id/key-results", get(list_key_results).post(create_key_result))
        .route("/api/goals/key-results/:id", put(update_key_result).delete(delete_key_result))
        .route("/api/goals/key-results/:id/check-in", post(create_check_in))
        .route("/api/goals/key-results/:id/history", get(get_check_in_history))
        .route("/api/goals/dashboard", get(get_dashboard))
        .route("/api/goals/alignment", get(get_alignment))
        .route("/api/goals/templates", get(list_templates))
        .route("/api/goals/ai/suggest", post(ai_suggest))
}
