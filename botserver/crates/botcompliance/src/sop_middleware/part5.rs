use super::*;

impl SopLogger {
    pub async fn search(&self, query: SopSearchQuery) -> SopSearchResult {
        let entries = self.entries.read().await;

        let filtered: Vec<_> = entries
            .iter()
            .filter(|e| {
                if let Some(ref cat) = query.category {
                    if &e.category != cat {
                        return false;
                    }
                }
                if let Some(ref sev) = query.severity {
                    if &e.severity != sev {
                        return false;
                    }
                }
                if let Some(ref out) = query.outcome {
                    if &e.outcome != out {
                        return false;
                    }
                }
                if let Some(uid) = query.user_id {
                    if e.user_id != Some(uid) {
                        return false;
                    }
                }
                if let Some(oid) = query.organization_id {
                    if e.organization_id != Some(oid) {
                        return false;
                    }
                }
                if let Some(ref rt) = query.resource_type {
                    if e.resource_type.as_ref() != Some(rt) {
                        return false;
                    }
                }
                if let Some(from) = query.from_date {
                    if e.timestamp < from {
                        return false;
                    }
                }
                if let Some(to) = query.to_date {
                    if e.timestamp > to {
                        return false;
                    }
                }
                if let Some(ref op) = query.operation_contains {
                    if !e.operation.to_lowercase().contains(&op.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref ctrl) = query.compliance_control {
                    if !e.compliance_controls.contains(ctrl) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total_count = filtered.len() as u64;
        let total_pages = ((total_count as f64) / (query.per_page as f64)).ceil() as u32;
        let start = ((query.page - 1) * query.per_page) as usize;
        let end = (start + query.per_page as usize).min(filtered.len());

        let page_entries = if start < filtered.len() {
            filtered[start..end].to_vec()
        } else {
            Vec::new()
        };

        SopSearchResult {
            entries: page_entries,
            total_count,
            page: query.page,
            per_page: query.per_page,
            total_pages,
        }
    }

    pub async fn get_statistics(
        &self,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> SopStatistics {
        let entries = self.entries.read().await;

        let filtered: Vec<_> = entries
            .iter()
            .filter(|e| e.timestamp >= from_date && e.timestamp <= to_date)
            .collect();

        let mut by_category: HashMap<String, u64> = HashMap::new();
        let mut by_severity: HashMap<String, u64> = HashMap::new();
        let mut by_outcome: HashMap<String, u64> = HashMap::new();
        let mut unique_users = std::collections::HashSet::new();
        let mut unique_orgs = std::collections::HashSet::new();

        for entry in &filtered {
            *by_category
                .entry(format!("{:?}", entry.category))
                .or_insert(0) += 1;
            *by_severity
                .entry(format!("{:?}", entry.severity))
                .or_insert(0) += 1;
            *by_outcome
                .entry(format!("{:?}", entry.outcome))
                .or_insert(0) += 1;

            if let Some(uid) = entry.user_id {
                unique_users.insert(uid);
            }
            if let Some(oid) = entry.organization_id {
                unique_orgs.insert(oid);
            }
        }

        SopStatistics {
            total_entries: filtered.len() as u64,
            entries_by_category: by_category,
            entries_by_severity: by_severity,
            entries_by_outcome: by_outcome,
            unique_users: unique_users.len() as u64,
            unique_organizations: unique_orgs.len() as u64,
            period_start: from_date,
            period_end: to_date,
        }
    }

    pub async fn export_for_audit(
        &self,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
        compliance_control: Option<&str>,
    ) -> Vec<SopLogEntry> {
        let entries = self.entries.read().await;

        entries
            .iter()
            .filter(|e| {
                if e.timestamp < from_date || e.timestamp > to_date {
                    return false;
                }
                if let Some(ctrl) = compliance_control {
                    if !e.compliance_controls.contains(&ctrl.to_string()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub async fn cleanup_old_entries(&self) {
        let cutoff = Utc::now() - chrono::Duration::days(self.retention_days);
        let mut entries = self.entries.write().await;
        entries.retain(|e| e.timestamp > cutoff);
    }

    pub(super) fn should_alert(&self, entry: &SopLogEntry) -> bool {
        matches!(entry.severity, SopSeverity::High | SopSeverity::Critical)
            || matches!(entry.outcome, SopOutcome::Denied | SopOutcome::Error)
            || matches!(
                entry.category,
                SopCategory::SecurityEvent | SopCategory::IncidentResponse
            )
    }

    async fn send_alert(&self, entry: &SopLogEntry) {
        log::warn!(
            "SOP ALERT: [{:?}] {:?} - {} - {}",
            entry.severity,
            entry.category,
            entry.operation,
            entry.description
        );
    }

    async fn log_to_persistent_storage(&self, entry: &SopLogEntry) {
        log::info!(
            "SOP LOG: [{:?}] {:?}/{:?} - {} - {}",
            entry.severity,
            entry.category,
            entry.outcome,
            entry.operation,
            entry.description
        );
    }

    async fn send_to_webhook(&self, webhook_url: &str, entry: &SopLogEntry) {
        log::debug!("Sending SOP entry to webhook: {}", webhook_url);
        let _ = entry;
    }

    async fn send_to_siem(&self, siem_endpoint: &str, entry: &SopLogEntry) {
        log::debug!("Sending SOP entry to SIEM: {}", siem_endpoint);
        let _ = entry;
    }
}
