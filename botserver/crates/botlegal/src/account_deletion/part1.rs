use super::*;

impl AccountDeletionService {
    pub fn new(grace_period_days: u32) -> Self {
        Self {
            grace_period_days,
            requests: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_deletion_request(
        &self,
        user_id: Uuid,
        email: String,
        organization_id: Option<Uuid>,
        request: CreateDeletionRequest,
    ) -> Result<AccountDeletionRequest, AccountDeletionError> {
        let requests = self.requests.read().await;
        let existing = requests.values().find(|r| {
            r.user_id == user_id
                && (r.status == DeletionStatus::Pending
                    || r.status == DeletionStatus::AwaitingConfirmation
                    || r.status == DeletionStatus::Confirmed)
        });

        if existing.is_some() {
            return Err(AccountDeletionError::RequestAlreadyExists);
        }
        drop(requests);

        let now = Utc::now();
        let scheduled_for = if request.immediate {
            now
        } else {
            now + chrono::Duration::days(i64::from(self.grace_period_days))
        };

        let deletion_request = AccountDeletionRequest {
            id: Uuid::new_v4(),
            user_id,
            organization_id,
            email: email.clone(),
            reason: request.reason,
            feedback: request.feedback,
            requested_at: now,
            scheduled_for,
            status: DeletionStatus::AwaitingConfirmation,
            confirmation_token: generate_confirmation_token(),
            confirmed_at: None,
            completed_at: None,
            deletion_report: None,
        };

        let mut requests = self.requests.write().await;
        requests.insert(deletion_request.id, deletion_request.clone());

        Ok(deletion_request)
    }

    pub async fn confirm_deletion(
        &self,
        request_id: Uuid,
        confirmation_token: &str,
    ) -> Result<AccountDeletionRequest, AccountDeletionError> {
        let mut requests = self.requests.write().await;
        let request = requests
            .get_mut(&request_id)
            .ok_or(AccountDeletionError::RequestNotFound)?;

        if request.status != DeletionStatus::AwaitingConfirmation {
            return Err(AccountDeletionError::InvalidStatus);
        }

        if request.confirmation_token != confirmation_token {
            return Err(AccountDeletionError::InvalidConfirmationToken);
        }

        request.status = DeletionStatus::Confirmed;
        request.confirmed_at = Some(Utc::now());

        Ok(request.clone())
    }

    pub async fn cancel_deletion(
        &self,
        request_id: Uuid,
        user_id: Uuid,
    ) -> Result<AccountDeletionRequest, AccountDeletionError> {
        let mut requests = self.requests.write().await;
        let request = requests
            .get_mut(&request_id)
            .ok_or(AccountDeletionError::RequestNotFound)?;

        if request.user_id != user_id {
            return Err(AccountDeletionError::Unauthorized);
        }

        if request.status == DeletionStatus::Completed
            || request.status == DeletionStatus::InProgress
        {
            return Err(AccountDeletionError::CannotCancel);
        }

        request.status = DeletionStatus::Cancelled;

        Ok(request.clone())
    }

    pub async fn execute_deletion(
        &self,
        request_id: Uuid,
    ) -> Result<DeletionReport, AccountDeletionError> {
        let mut requests = self.requests.write().await;
        let request = requests
            .get_mut(&request_id)
            .ok_or(AccountDeletionError::RequestNotFound)?;

        if request.status != DeletionStatus::Confirmed {
            return Err(AccountDeletionError::NotConfirmed);
        }

        request.status = DeletionStatus::InProgress;
        let user_id = request.user_id;
        drop(requests);

        let started_at = Utc::now();
        let mut data_deleted = Vec::new();
        let mut data_retained = Vec::new();
        let mut total_records: u64 = 0;

        let profile_deleted = self.delete_profile_data(user_id).await;
        total_records += profile_deleted;
        data_deleted.push(DeletedDataCategory {
            category: DataCategory::Profile,
            records_deleted: profile_deleted,
            storage_freed_bytes: 0,
        });

        let sessions_deleted = self.delete_sessions(user_id).await;
        total_records += sessions_deleted;
        data_deleted.push(DeletedDataCategory {
            category: DataCategory::Sessions,
            records_deleted: sessions_deleted,
            storage_freed_bytes: 0,
        });

        let messages_deleted = self.delete_messages(user_id).await;
        total_records += messages_deleted;
        data_deleted.push(DeletedDataCategory {
            category: DataCategory::Messages,
            records_deleted: messages_deleted,
            storage_freed_bytes: 0,
        });

        let files_result = self.delete_files(user_id).await;
        total_records += files_result.0;
        data_deleted.push(DeletedDataCategory {
            category: DataCategory::Files,
            records_deleted: files_result.0,
            storage_freed_bytes: files_result.1,
        });

        let consents_deleted = self.delete_consents(user_id).await;
        total_records += consents_deleted;
        data_deleted.push(DeletedDataCategory {
            category: DataCategory::Consents,
            records_deleted: consents_deleted,
            storage_freed_bytes: 0,
        });

        let api_keys_deleted = self.delete_api_keys(user_id).await;
        total_records += api_keys_deleted;
        data_deleted.push(DeletedDataCategory {
            category: DataCategory::ApiKeys,
            records_deleted: api_keys_deleted,
            storage_freed_bytes: 0,
        });

        let preferences_deleted = self.delete_preferences(user_id).await;
        total_records += preferences_deleted;
        data_deleted.push(DeletedDataCategory {
            category: DataCategory::Preferences,
            records_deleted: preferences_deleted,
            storage_freed_bytes: 0,
        });

        data_retained.push(RetainedDataCategory {
            category: DataCategory::AuditLogs,
            reason: RetentionReason::LegalObligation,
            retention_period: Some("7 years".to_string()),
            will_be_deleted_at: Some(Utc::now() + chrono::Duration::days(2555)),
        });

        data_retained.push(RetainedDataCategory {
            category: DataCategory::Invoices,
            reason: RetentionReason::TaxRecords,
            retention_period: Some("7 years".to_string()),
            will_be_deleted_at: Some(Utc::now() + chrono::Duration::days(2555)),
        });

        let completed_at = Utc::now();
        let report = DeletionReport {
            user_id,
            started_at,
            completed_at,
            data_deleted,
            data_retained,
            total_records_deleted: total_records,
            confirmation_number: format!("DEL-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        };

        let mut requests = self.requests.write().await;
        if let Some(request) = requests.get_mut(&request_id) {
            request.status = DeletionStatus::Completed;
            request.completed_at = Some(completed_at);
            request.deletion_report = Some(report.clone());
        }

        Ok(report)
    }

    pub async fn get_deletion_request(
        &self,
        request_id: Uuid,
    ) -> Option<AccountDeletionRequest> {
        let requests = self.requests.read().await;
        requests.get(&request_id).cloned()
    }

    pub async fn get_user_deletion_requests(
        &self,
        user_id: Uuid,
    ) -> Vec<AccountDeletionRequest> {
        let requests = self.requests.read().await;
        requests
            .values()
            .filter(|r| r.user_id == user_id)
            .cloned()
            .collect()
    }

    pub async fn get_pending_deletions(&self) -> Vec<AccountDeletionRequest> {
        let requests = self.requests.read().await;
        let now = Utc::now();
        requests
            .values()
            .filter(|r| r.status == DeletionStatus::Confirmed && r.scheduled_for <= now)
            .cloned()
            .collect()
    }

    async fn delete_profile_data(&self, _user_id: Uuid) -> u64 {
        1
    }

    async fn delete_sessions(&self, _user_id: Uuid) -> u64 {
        5
    }

    async fn delete_messages(&self, _user_id: Uuid) -> u64 {
        100
    }

    async fn delete_files(&self, _user_id: Uuid) -> (u64, u64) {
        (25, 1024 * 1024 * 50)
    }

    async fn delete_consents(&self, _user_id: Uuid) -> u64 {
        3
    }

    async fn delete_api_keys(&self, _user_id: Uuid) -> u64 {
        2
    }

    async fn delete_preferences(&self, _user_id: Uuid) -> u64 {
        10
    }

    pub fn grace_period_days(&self) -> u32 {
        self.grace_period_days
    }
}
impl Default for AccountDeletionService {
    fn default() -> Self {
        Self::new(30)
    }
}
pub(super) fn generate_confirmation_token() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    Uuid::new_v4().hash(&mut hasher);
    Utc::now().timestamp_nanos_opt().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
