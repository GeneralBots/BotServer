use super::*;

impl std::fmt::Display for SyncErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncErrorType::FetchError => write!(f, "fetch_error"),
            SyncErrorType::ParseError => write!(f, "parse_error"),
            SyncErrorType::TransformError => write!(f, "transform_error"),
            SyncErrorType::InsertError => write!(f, "insert_error"),
            SyncErrorType::UpdateError => write!(f, "update_error"),
            SyncErrorType::ConflictError => write!(f, "conflict_error"),
            SyncErrorType::ValidationError => write!(f, "validation_error"),
            SyncErrorType::ConnectionError => write!(f, "connection_error"),
            SyncErrorType::TimeoutError => write!(f, "timeout_error"),
            SyncErrorType::RateLimitError => write!(f, "rate_limit_error"),
        }
    }
}
