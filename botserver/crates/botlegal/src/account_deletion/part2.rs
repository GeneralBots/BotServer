use super::*;

impl std::fmt::Display for AccountDeletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestNotFound => write!(f, "Deletion request not found"),
            Self::RequestAlreadyExists => write!(f, "A deletion request already exists for this account"),
            Self::InvalidStatus => write!(f, "Invalid request status for this operation"),
            Self::InvalidConfirmationToken => write!(f, "Invalid confirmation token"),
            Self::Unauthorized => write!(f, "Not authorized to perform this action"),
            Self::CannotCancel => write!(f, "Cannot cancel deletion request at this stage"),
            Self::NotConfirmed => write!(f, "Deletion request has not been confirmed"),
            Self::DeletionFailed(msg) => write!(f, "Deletion failed: {msg}"),
        }
    }
}
impl std::error::Error for AccountDeletionError {}
