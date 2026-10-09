use super::*;

impl std::fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictResolution::Update => write!(f, "update"),
            ConflictResolution::Skip => write!(f, "skip"),
            ConflictResolution::Error => write!(f, "error"),
            ConflictResolution::Upsert => write!(f, "upsert"),
        }
    }
}
