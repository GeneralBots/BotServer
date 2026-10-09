use super::*;

impl std::fmt::Display for TimeSeriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeSeriesError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            TimeSeriesError::WriteError(msg) => write!(f, "Write error: {}", msg),
            TimeSeriesError::QueryError(msg) => write!(f, "Query error: {}", msg),
            TimeSeriesError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}
impl std::error::Error for TimeSeriesError {}
