use super::*;

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Pending => write!(f, "pending"),
            SyncStatus::Running => write!(f, "running"),
            SyncStatus::Completed => write!(f, "completed"),
            SyncStatus::Failed => write!(f, "failed"),
            SyncStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}
