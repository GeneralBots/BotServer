use super::*;

impl Default for SyncResult {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            success: false,
            records_fetched: 0,
            records_inserted: 0,
            records_updated: 0,
            records_skipped: 0,
            records_failed: 0,
            pages_processed: 0,
            duration_ms: 0,
            errors: Vec::new(),
            started_at: now,
            completed_at: now,
        }
    }
}
