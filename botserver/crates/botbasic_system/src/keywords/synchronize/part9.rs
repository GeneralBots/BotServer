use super::*;

impl std::fmt::Display for SynchronizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SynchronizeError::FetchFailed(msg) => write!(f, "Fetch failed: {}", msg),
            SynchronizeError::ParseFailed(msg) => write!(f, "Parse failed: {}", msg),
            SynchronizeError::TransformFailed(msg) => write!(f, "Transform failed: {}", msg),
            SynchronizeError::InsertFailed(msg) => write!(f, "Insert failed: {}", msg),
            SynchronizeError::UpdateFailed(msg) => write!(f, "Update failed: {}", msg),
            SynchronizeError::ConflictDetected(key) => write!(f, "Conflict detected for key: {}", key),
            SynchronizeError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            SynchronizeError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            SynchronizeError::Timeout => write!(f, "Operation timed out"),
            SynchronizeError::RateLimited => write!(f, "Rate limited by remote server"),
            SynchronizeError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}
impl std::error::Error for SynchronizeError {}
impl SynchronizeError {
    pub fn error_type(&self) -> SyncErrorType {
        match self {
            SynchronizeError::FetchFailed(_) => SyncErrorType::FetchError,
            SynchronizeError::ParseFailed(_) => SyncErrorType::ParseError,
            SynchronizeError::TransformFailed(_) => SyncErrorType::TransformError,
            SynchronizeError::InsertFailed(_) => SyncErrorType::InsertError,
            SynchronizeError::UpdateFailed(_) => SyncErrorType::UpdateError,
            SynchronizeError::ConflictDetected(_) => SyncErrorType::ConflictError,
            SynchronizeError::ValidationFailed(_) => SyncErrorType::ValidationError,
            SynchronizeError::ConnectionFailed(_) => SyncErrorType::ConnectionError,
            SynchronizeError::Timeout => SyncErrorType::TimeoutError,
            SynchronizeError::RateLimited => SyncErrorType::RateLimitError,
            SynchronizeError::InvalidConfig(_) => SyncErrorType::ValidationError,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SynchronizeError::Timeout
                | SynchronizeError::RateLimited
                | SynchronizeError::ConnectionFailed(_)
        )
    }
}
pub fn parse_synchronize_args(args: &[String]) -> Result<SynchronizeConfig, SynchronizeError> {
    if args.len() < 5 {
        return Err(SynchronizeError::InvalidConfig(
            "SYNCHRONIZE requires 5 arguments: endpoint, tableName, keyField, pageParam, limitParam".to_string(),
        ));
    }

    Ok(SynchronizeConfig {
        endpoint: args[0].clone(),
        table_name: args[1].clone(),
        key_field: args[2].clone(),
        page_param: args[3].clone(),
        limit_param: args[4].clone(),
        page_size: None,
        headers: None,
        transform: None,
        conflict_resolution: None,
        batch_size: None,
        dry_run: None,
    })
}
pub fn generate_sync_report(result: &SyncResult) -> String {
    let mut report = String::new();

    report.push_str("=== SYNCHRONIZE Report ===\n");
    report.push_str(&format!("Status: {}\n", if result.success { "SUCCESS" } else { "FAILED" }));
    report.push_str(&format!("Duration: {}ms\n", result.duration_ms));
    report.push_str(&format!("Pages Processed: {}\n", result.pages_processed));
    report.push_str("\n--- Records ---\n");
    report.push_str(&format!("Fetched: {}\n", result.records_fetched));
    report.push_str(&format!("Inserted: {}\n", result.records_inserted));
    report.push_str(&format!("Updated: {}\n", result.records_updated));
    report.push_str(&format!("Skipped: {}\n", result.records_skipped));
    report.push_str(&format!("Failed: {}\n", result.records_failed));

    if !result.errors.is_empty() {
        report.push_str("\n--- Errors ---\n");
        for (i, error) in result.errors.iter().enumerate().take(10) {
            report.push_str(&format!(
                "{}. [{}] {}\n",
                i + 1,
                error.error_type,
                error.message
            ));
        }
        if result.errors.len() > 10 {
            report.push_str(&format!("... and {} more errors\n", result.errors.len() - 10));
        }
    }

    report.push_str("=========================\n");
    report
}
