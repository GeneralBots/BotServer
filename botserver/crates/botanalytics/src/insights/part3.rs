use super::*;

pub async fn handle_track_usage(
    State(pool): State<Arc<DbPool>>,
    Json(req): Json<TrackUsageRequest>,
) -> Result<Json<AppUsage>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let usage = service.track_usage(user_id, req).await?;
    Ok(Json(usage))
}
pub async fn handle_get_daily(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<InsightsQuery>,
) -> Result<Json<DailyInsights>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let date = query.start_date.unwrap_or_else(|| Utc::now().date_naive());
    let insights = service.get_daily_insights(user_id, date).await?;
    Ok(Json(insights))
}
pub async fn handle_get_weekly(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<InsightsQuery>,
) -> Result<Json<WeeklyInsights>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let date = query.start_date.unwrap_or_else(|| Utc::now().date_naive());
    let insights = service.get_weekly_insights(user_id, date).await?;
    Ok(Json(insights))
}
pub async fn handle_get_trends(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<InsightsQuery>,
) -> Result<Json<Vec<DailyInsights>>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let end_date = query.end_date.unwrap_or_else(|| Utc::now().date_naive());
    let start_date = query.start_date.unwrap_or_else(|| end_date - Duration::days(30));
    let trends = service.get_trends(user_id, start_date, end_date).await?;
    Ok(Json(trends))
}
pub async fn handle_get_recommendations(
    State(pool): State<Arc<DbPool>>,
) -> Result<Json<Vec<WellbeingRecommendation>>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let recommendations = service.generate_recommendations(user_id).await;
    Ok(Json(recommendations))
}
pub async fn handle_get_settings(
    State(pool): State<Arc<DbPool>>,
) -> Result<Json<InsightsSettings>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let settings = service.get_settings(user_id).await?;
    Ok(Json(settings))
}
pub async fn handle_update_settings(
    State(pool): State<Arc<DbPool>>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<InsightsSettings>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let settings = service.update_settings(user_id, req).await?;
    Ok(Json(settings))
}
pub async fn handle_update_focus_mode(
    State(pool): State<Arc<DbPool>>,
    Json(req): Json<UpdateFocusModeRequest>,
) -> Result<Json<FocusMode>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let focus_mode = service.update_focus_mode(user_id, req).await?;
    Ok(Json(focus_mode))
}
pub async fn handle_get_app_breakdown(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<InsightsQuery>,
) -> Result<Json<Vec<AppUsageSummary>>, InsightsError> {
    let service = InsightsService::new((*pool).clone());
    let user_id = Uuid::nil();
    let date = query.start_date.unwrap_or_else(|| Utc::now().date_naive());
    let breakdown = service.get_app_breakdown(user_id, date).await?;
    Ok(Json(breakdown))
}
pub fn configure_insights_routes() -> Router<Arc<DbPool>> {
    Router::new()
        .route("/api/insights/track", post(handle_track_usage))
        .route("/api/insights/daily", get(handle_get_daily))
        .route("/api/insights/weekly", get(handle_get_weekly))
        .route("/api/insights/trends", get(handle_get_trends))
        .route("/api/insights/recommendations", get(handle_get_recommendations))
        .route("/api/insights/settings", get(handle_get_settings).put(handle_update_settings))
        .route("/api/insights/focus-mode", put(handle_update_focus_mode))
        .route("/api/insights/apps", get(handle_get_app_breakdown))
}
