use super::*;

pub(super) async fn preview_cleanup_handler(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<PreviewQuery>,
) -> Result<Json<CleanupPreview>, CleanupError> {
    let service = CleanupService::new(pool);

    let categories = query.categories.map(|s| {
        s.split(',')
            .filter_map(|c| match c.trim() {
                "bot_history" => Some(CleanupCategory::BotHistory),
                "chat_sessions" => Some(CleanupCategory::ChatSessions),
                "system_logs" => Some(CleanupCategory::SystemLogs),
                "access_logs" => Some(CleanupCategory::AccessLogs),
                "debug_logs" => Some(CleanupCategory::DebugLogs),
                "temp_files" => Some(CleanupCategory::TempFiles),
                "cache_data" => Some(CleanupCategory::CacheData),
                "analytics_raw" => Some(CleanupCategory::AnalyticsRaw),
                _ => None,
            })
            .collect()
    });

    let preview = service
        .preview_cleanup(query.organization_id, query.retention_days, categories)
        .await?;

    Ok(Json(preview))
}
pub(super) async fn execute_cleanup_handler(
    State(pool): State<Arc<DbPool>>,
    Json(request): Json<ExecuteRequest>,
) -> Result<Json<CleanupResult>, CleanupError> {
    let service = CleanupService::new(pool);

    let categories = request.categories.map(|cats| {
        cats.iter()
            .filter_map(|c| match c.as_str() {
                "bot_history" => Some(CleanupCategory::BotHistory),
                "chat_sessions" => Some(CleanupCategory::ChatSessions),
                "system_logs" => Some(CleanupCategory::SystemLogs),
                "access_logs" => Some(CleanupCategory::AccessLogs),
                "debug_logs" => Some(CleanupCategory::DebugLogs),
                "temp_files" => Some(CleanupCategory::TempFiles),
                "cache_data" => Some(CleanupCategory::CacheData),
                "analytics_raw" => Some(CleanupCategory::AnalyticsRaw),
                _ => None,
            })
            .collect()
    });

    let result = service
        .execute_cleanup(
            request.organization_id,
            request.retention_days,
            categories,
            request.performed_by,
            CleanupTrigger::Manual,
        )
        .await?;

    Ok(Json(result))
}
pub(super) async fn storage_usage_handler(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<StorageQuery>,
) -> Result<Json<StorageUsage>, CleanupError> {
    let service = CleanupService::new(pool);
    let usage = service.get_storage_usage(query.organization_id).await?;
    Ok(Json(usage))
}
pub(super) async fn cleanup_history_handler(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<CleanupHistory>>, CleanupError> {
    let service = CleanupService::new(pool);
    let history = service
        .get_cleanup_history(query.organization_id, query.limit)
        .await?;
    Ok(Json(history))
}
pub(super) async fn get_config_handler(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<StorageQuery>,
) -> Result<Json<CleanupConfig>, CleanupError> {
    let service = CleanupService::new(pool);
    let config = service.get_cleanup_config(query.organization_id).await?;
    Ok(Json(config))
}
pub(super) async fn save_config_handler(
    State(pool): State<Arc<DbPool>>,
    Json(config): Json<CleanupConfig>,
) -> Result<StatusCode, CleanupError> {
    let service = CleanupService::new(pool);
    service.save_cleanup_config(&config).await?;
    Ok(StatusCode::OK)
}
