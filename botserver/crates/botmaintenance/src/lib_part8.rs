use super::*;

impl std::fmt::Display for CleanupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseConnection => write!(f, "Database connection failed"),
            Self::CleanupFailed(cat) => write!(f, "Cleanup failed for {cat}"),
            Self::ConfigSaveFailed => write!(f, "Failed to save configuration"),
            Self::HistorySaveFailed => write!(f, "Failed to save history"),
            Self::OrganizationNotFound => write!(f, "Organization not found"),
            Self::RetentionTooShort(min) => write!(f, "Retention must be at least {min} days"),
            Self::RetentionTooLong(max) => write!(f, "Retention cannot exceed {max} days"),
            Self::InvalidCategory(cat) => write!(f, "Invalid category: {cat}"),
        }
    }
}
impl std::error::Error for CleanupError {}
impl IntoResponse for CleanupError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::OrganizationNotFound => StatusCode::NOT_FOUND,
            Self::RetentionTooShort(_) | Self::RetentionTooLong(_) | Self::InvalidCategory(_) => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}
pub fn create_cleanup_tables_migration() -> &'static str {
    r#"
        CREATE TABLE IF NOT EXISTS cleanup_config (
        organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
        retention_days BIGINT NOT NULL DEFAULT 180,
        enabled_categories TEXT NOT NULL DEFAULT 'bot_history,chat_sessions,system_logs,debug_logs,temp_files,cache_data',
        auto_cleanup_enabled BOOLEAN NOT NULL DEFAULT FALSE,
        auto_cleanup_schedule TEXT,
        storage_alert_threshold INTEGER NOT NULL DEFAULT 80,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS cleanup_history (
        id TEXT PRIMARY KEY,
        organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        performed_at TIMESTAMPTZ NOT NULL,
        performed_by UUID REFERENCES users(id),
        trigger TEXT NOT NULL,
        categories_cleaned TEXT NOT NULL,
        records_deleted BIGINT NOT NULL,
        bytes_freed BIGINT NOT NULL,
        retention_days_used BIGINT NOT NULL,
        duration_ms BIGINT NOT NULL,
        success BOOLEAN NOT NULL,
        error_message TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_cleanup_history_org ON cleanup_history(organization_id);
        CREATE INDEX IF NOT EXISTS idx_cleanup_history_performed ON cleanup_history(performed_at DESC);
        "#
}
pub fn cleanup_routes() -> Router<Arc<DbPool>> {
    Router::new()
        .route("/preview", get(preview_cleanup_handler))
        .route("/execute", post(execute_cleanup_handler))
        .route("/storage", get(storage_usage_handler))
        .route("/history", get(cleanup_history_handler))
        .route("/config", get(get_config_handler))
        .route("/config", post(save_config_handler))
}
