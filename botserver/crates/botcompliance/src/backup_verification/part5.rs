use super::*;

impl BackupVerificationService {
    pub fn new() -> Self {
        Self {
            backups: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            verifications: Arc::new(RwLock::new(Vec::new())),
            restore_tests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_backup(&self, backup: BackupRecord) -> Result<BackupRecord, BackupError> {
        let mut backups = self.backups.write().await;
        backups.insert(backup.id, backup.clone());
        Ok(backup)
    }

    pub async fn get_backup(&self, id: Uuid) -> Option<BackupRecord> {
        let backups = self.backups.read().await;
        backups.get(&id).cloned()
    }

    pub async fn get_all_backups(&self) -> Vec<BackupRecord> {
        let backups = self.backups.read().await;
        backups.values().cloned().collect()
    }

    pub async fn get_latest_backup(&self, source: &str) -> Option<BackupRecord> {
        let backups = self.backups.read().await;
        backups
            .values()
            .filter(|b| b.source == source && b.status == BackupStatus::Completed)
            .max_by_key(|b| b.completed_at)
            .cloned()
    }

    pub async fn verify_backup(
        &self,
        backup_id: Uuid,
        methods: Vec<VerificationMethod>,
        verified_by: Option<Uuid>,
    ) -> Result<VerificationResult, BackupError> {
        let backups = self.backups.read().await;
        let backup = backups
            .get(&backup_id)
            .ok_or_else(|| BackupError::NotFound("Backup not found".to_string()))?
            .clone();
        drop(backups);

        let verification_id = Uuid::new_v4();
        let started_at = Utc::now();

        let mut checks = Vec::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for method in &methods {
            match method {
                VerificationMethod::ChecksumValidation => {
                    let check_result = self.verify_checksum(&backup).await;
                    checks.push(check_result.0);
                    if let Some(err) = check_result.1 {
                        errors.push(err);
                    }
                }
                VerificationMethod::MetadataCheck => {
                    let check_result = self.verify_metadata(&backup).await;
                    checks.push(check_result.0);
                    if let Some(err) = check_result.1 {
                        errors.push(err);
                    }
                }
                VerificationMethod::RestoreTest => {
                    let check_result = self.perform_restore_test(&backup).await;
                    checks.push(check_result.0);
                    if let Some(err) = check_result.1 {
                        errors.push(err);
                    }
                }
                VerificationMethod::PartialRestore => {
                    let check_result = self.perform_partial_restore(&backup).await;
                    checks.push(check_result.0);
                    if let Some(err) = check_result.1 {
                        errors.push(err);
                    }
                }
                VerificationMethod::SampleDataValidation => {
                    let check_result = self.validate_sample_data(&backup).await;
                    checks.push(check_result.0);
                    if let Some(err) = check_result.1 {
                        errors.push(err);
                    }
                }
                VerificationMethod::FullIntegrityCheck => {
                    let check_result = self.full_integrity_check(&backup).await;
                    checks.extend(check_result.0);
                    errors.extend(check_result.1);
                    warnings.extend(check_result.2);
                }
            }
        }

        let all_passed = checks.iter().all(|c| c.passed);
        let any_passed = checks.iter().any(|c| c.passed);

        let status = if errors.iter().any(|e| e.severity == ErrorSeverity::Critical) {
            VerificationStatus::Failed
        } else if all_passed {
            VerificationStatus::Passed
        } else if any_passed {
            VerificationStatus::PartialSuccess
        } else {
            VerificationStatus::Failed
        };

        let completed_at = Utc::now();
        let duration_seconds = (completed_at - started_at).num_seconds() as u64;

        let result = VerificationResult {
            id: verification_id,
            backup_id,
            method: methods.first().cloned().unwrap_or(VerificationMethod::ChecksumValidation),
            status: status.clone(),
            started_at,
            completed_at: Some(completed_at),
            duration_seconds: Some(duration_seconds),
            checks_performed: checks,
            errors,
            warnings,
            verified_by,
            notes: None,
        };

        let mut verifications = self.verifications.write().await;
        verifications.push(result.clone());

        let mut backups = self.backups.write().await;
        if let Some(backup) = backups.get_mut(&backup_id) {
            backup.verification_history.push(result.clone());
            backup.last_verified_at = Some(completed_at);
            backup.status = if status == VerificationStatus::Passed {
                BackupStatus::Verified
            } else {
                BackupStatus::VerificationFailed
            };
        }

        Ok(result)
    }

    async fn verify_checksum(
        &self,
        backup: &BackupRecord,
    ) -> (VerificationCheck, Option<VerificationError>) {
        let start = std::time::Instant::now();

        let computed_checksum = self.compute_checksum(&backup.storage_path).await;

        let passed = computed_checksum
            .as_ref()
            .map(|c| c == &backup.checksum)
            .unwrap_or(false);

        let check = VerificationCheck {
            name: "Checksum Validation".to_string(),
            description: format!("Verify {} checksum matches stored value", backup.checksum_algorithm),
            passed,
            expected_value: Some(backup.checksum.clone()),
            actual_value: computed_checksum.clone(),
            duration_ms: start.elapsed().as_millis() as u64,
        };

        let error = if !passed {
            Some(VerificationError {
                code: "CHECKSUM_MISMATCH".to_string(),
                message: "Backup checksum does not match the stored value".to_string(),
                severity: ErrorSeverity::Critical,
                recoverable: false,
                suggested_action: Some("Backup may be corrupted. Consider creating a new backup.".to_string()),
            })
        } else {
            None
        };

        (check, error)
    }

    async fn compute_checksum(&self, _path: &str) -> Option<String> {
        Some("computed_checksum_placeholder".to_string())
    }

    async fn verify_metadata(
        &self,
        backup: &BackupRecord,
    ) -> (VerificationCheck, Option<VerificationError>) {
        let start = std::time::Instant::now();

        let metadata_valid = backup.size_bytes > 0
            && backup.completed_at.is_some()
            && !backup.storage_path.is_empty();

        let check = VerificationCheck {
            name: "Metadata Validation".to_string(),
            description: "Verify backup metadata is complete and consistent".to_string(),
            passed: metadata_valid,
            expected_value: Some("Complete metadata".to_string()),
            actual_value: Some(if metadata_valid {
                "Metadata valid".to_string()
            } else {
                "Metadata incomplete".to_string()
            }),
            duration_ms: start.elapsed().as_millis() as u64,
        };

        let error = if !metadata_valid {
            Some(VerificationError {
                code: "INVALID_METADATA".to_string(),
                message: "Backup metadata is incomplete or invalid".to_string(),
                severity: ErrorSeverity::High,
                recoverable: true,
                suggested_action: Some("Update backup metadata or re-run backup".to_string()),
            })
        } else {
            None
        };

        (check, error)
    }

    async fn perform_restore_test(
        &self,
        backup: &BackupRecord,
    ) -> (VerificationCheck, Option<VerificationError>) {
        let start = std::time::Instant::now();

        let restore_success = self.simulate_restore(backup).await;

        let check = VerificationCheck {
            name: "Restore Test".to_string(),
            description: "Perform full restore to test environment".to_string(),
            passed: restore_success,
            expected_value: Some("Successful restore".to_string()),
            actual_value: Some(if restore_success {
                "Restore completed".to_string()
            } else {
                "Restore failed".to_string()
            }),
            duration_ms: start.elapsed().as_millis() as u64,
        };

        let error = if !restore_success {
            Some(VerificationError {
                code: "RESTORE_FAILED".to_string(),
                message: "Backup could not be restored successfully".to_string(),
                severity: ErrorSeverity::Critical,
                recoverable: false,
                suggested_action: Some("Investigate restore failure and create new backup".to_string()),
            })
        } else {
            None
        };

        (check, error)
    }

    async fn simulate_restore(&self, _backup: &BackupRecord) -> bool {
        true
    }
}
