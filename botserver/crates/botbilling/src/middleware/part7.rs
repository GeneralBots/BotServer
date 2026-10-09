use super::*;

impl IntoResponse for StorageQuotaExceededResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "storage_quota_exceeded",
            "message": self.message,
            "code": "STORAGE_QUOTA_EXCEEDED",
            "current_usage_bytes": self.current_usage,
            "limit_bytes": self.limit,
            "upgrade_url": "/billing/upgrade"
        });

        (
            StatusCode::PAYLOAD_TOO_LARGE,
            [
                ("Content-Type", "application/json"),
                ("X-Storage-Quota-Exceeded", "true"),
            ],
            Json(body),
        )
            .into_response()
    }
}
pub fn create_quota_middleware_state(quota_manager: Arc<QuotaManager>) -> Arc<QuotaMiddlewareState> {
    Arc::new(QuotaMiddlewareState::new(quota_manager))
}
pub async fn toggle_saas_mode(state: &QuotaMiddlewareState, enabled: bool) {
    state.set_enabled(enabled).await;
    if enabled {
        tracing::info!("SaaS quota enforcement enabled");
    } else {
        tracing::info!("SaaS quota enforcement disabled (local mode)");
    }
}
