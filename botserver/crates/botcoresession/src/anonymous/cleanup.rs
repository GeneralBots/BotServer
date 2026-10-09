//! Background cleanup job for expired anonymous sessions.
use std::sync::Arc;

use super::manager::AnonymousSessionManager;

pub async fn session_cleanup_job(
    manager: Arc<AnonymousSessionManager>,
    interval_seconds: u64,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));

    loop {
        interval.tick().await;
        let cleaned = manager.cleanup_expired_sessions().await;
        if cleaned > 0 {
            log::info!("Cleaned up {} expired anonymous sessions", cleaned);
        }
    }
}
