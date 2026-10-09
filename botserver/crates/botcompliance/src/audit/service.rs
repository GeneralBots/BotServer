use super::*;

impl AuditService {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::new())),
            trails: Arc::new(RwLock::new(HashMap::new())),
            retention_policies: Arc::new(RwLock::new(vec![
                RetentionPolicy {
                    name: "Security Events".to_string(),
                    retention_days: 365,
                    event_types: vec![
                        AuditEventType::SecurityAlert,
                        AuditEventType::ComplianceViolation,
                    ],
                    severity_threshold: Some(AuditSeverity::Warning),
                    archive_enabled: true,
                },
                RetentionPolicy {
                    name: "Access Logs".to_string(),
                    retention_days: 90,
                    event_types: vec![
                        AuditEventType::UserLogin,
                        AuditEventType::UserLogout,
                        AuditEventType::DataAccess,
                    ],
                    severity_threshold: None,
                    archive_enabled: false,
                },
            ])),
            max_events,
        }
    }

    pub async fn log_event(
        &self,
        event_type: AuditEventType,
        severity: AuditSeverity,
        user_id: Option<Uuid>,
        action: String,
        outcome: AuditOutcome,
        details: HashMap<String, String>,
    ) -> Result<Uuid> {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: event_type.clone(),
            severity: severity.clone(),
            user_id,
            session_id: None,
            ip_address: None,
            resource_id: None,
            action,
            outcome,
            details,
            metadata: serde_json::json!({}),
        };

        let event_id = event.id;

        {
            let mut events = self.events.write().await;
            events.push_back(event);

            while events.len() > self.max_events {
                events.pop_front();
            }
        }

        log::info!("Audit event logged: {event_id} - {event_type:?} - {severity:?}");

        Ok(event_id)
    }

    pub async fn create_trail(&self, name: String, tags: Vec<String>) -> Result<Uuid> {
        let trail = AuditTrail {
            trail_id: Uuid::new_v4(),
            name,
            started_at: Utc::now(),
            ended_at: None,
            events: Vec::new(),
            summary: String::new(),
            tags,
        };

        let trail_id = trail.trail_id;
        {
            let mut trails = self.trails.write().await;
            trails.insert(trail_id, trail);
        }

        Ok(trail_id)
    }

    pub async fn add_to_trail(&self, trail_id: Uuid, event_id: Uuid) -> Result<()> {
        let mut trails = self.trails.write().await;
        let trail = trails
            .get_mut(&trail_id)
            .ok_or_else(|| anyhow!("Trail not found"))?;

        if trail.ended_at.is_some() {
            return Err(anyhow!("Trail already ended"));
        }

        trail.events.push(event_id);
        Ok(())
    }

    pub async fn end_trail(&self, trail_id: Uuid, summary: String) -> Result<()> {
        let mut trails = self.trails.write().await;
        let trail = trails
            .get_mut(&trail_id)
            .ok_or_else(|| anyhow!("Trail not found"))?;

        trail.ended_at = Some(Utc::now());
        trail.summary = summary;

        Ok(())
    }

    pub async fn query_events(&self, filter: AuditFilter) -> Result<Vec<AuditEvent>> {
        let filtered: Vec<AuditEvent> = {
            let events = self.events.read().await;
            events
                .iter()
                .filter(|e| filter.matches(e))
                .cloned()
                .collect()
        };

        Ok(filtered)
    }

    pub async fn get_statistics(
        &self,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    ) -> AuditStatistics {
        let since = since.unwrap_or_else(|| Utc::now() - Duration::days(30));
        let until = until.unwrap_or_else(Utc::now);

        let filtered_events: Vec<AuditEvent> = {
            let events = self.events.read().await;
            events
                .iter()
                .filter(|e| e.timestamp >= since && e.timestamp <= until)
                .cloned()
                .collect()
        };

        let mut events_by_type = HashMap::new();
        let mut events_by_severity = HashMap::new();
        let mut events_by_outcome = HashMap::new();
        let mut unique_users = std::collections::HashSet::new();

        for event in &filtered_events {
            *events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;
            *events_by_severity
                .entry(event.severity.clone())
                .or_insert(0) += 1;
            *events_by_outcome.entry(event.outcome.clone()).or_insert(0) += 1;

            if let Some(user_id) = event.user_id {
                unique_users.insert(user_id);
            }
        }

        AuditStatistics {
            total_events: filtered_events.len(),
            events_by_type,
            events_by_severity,
            events_by_outcome,
            unique_users: unique_users.len(),
            time_range: (since, until),
        }
    }

    pub async fn apply_retention_policies(&self) -> Result<usize> {
        let policies = {
            let p = self.retention_policies.read().await;
            p.clone()
        };

        let mut events = self.events.write().await;
        let now = Utc::now();
        let mut removed_count = 0;

        for policy in policies.iter() {
            let cutoff = now - Duration::days(policy.retention_days);

            let initial_len = events.len();
            events.retain(|e| {
                if !policy.event_types.contains(&e.event_type) {
                    return true;
                }

                if let Some(threshold) = &policy.severity_threshold {
                    if e.severity < *threshold {
                        return true;
                    }
                }

                e.timestamp >= cutoff
            });

            removed_count += initial_len - events.len();
        }

        log::info!("Applied retention policies, removed {removed_count} events");
        Ok(removed_count)
    }

    pub async fn export_logs(
        &self,
        format: ExportFormat,
        filter: Option<AuditFilter>,
    ) -> Result<Vec<u8>> {
        let events = self.query_events(filter.unwrap_or_default()).await?;

        match format {
            ExportFormat::Json => {
                let json = serde_json::to_vec_pretty(&events)?;
                Ok(json)
            }
            ExportFormat::Csv => {
                let mut csv_writer = csv::Writer::from_writer(vec![]);

                csv_writer.write_record([
                    "ID",
                    "Timestamp",
                    "Type",
                    "Severity",
                    "User",
                    "Action",
                    "Outcome",
                ])?;

                for event in events {
                    csv_writer.write_record([
                        event.id.to_string(),
                        event.timestamp.to_rfc3339(),
                        format!("{:?}", event.event_type),
                        format!("{:?}", event.severity),
                        event.user_id.map(|u| u.to_string()).unwrap_or_default(),
                        event.action,
                        format!("{:?}", event.outcome),
                    ])?;
                }

                Ok(csv_writer.into_inner()?)
            }
        }
    }

    pub async fn get_compliance_report(&self) -> ComplianceReport {
        let stats = self.get_statistics(None, None).await;

        let (security_incidents, compliance_violations, failed_logins) = {
            let events = self.events.read().await;

            let security_incidents = events
                .iter()
                .filter(|e| e.event_type == AuditEventType::SecurityAlert)
                .count();

            let compliance_violations = events
                .iter()
                .filter(|e| e.event_type == AuditEventType::ComplianceViolation)
                .count();

            let failed_logins = events
                .iter()
                .filter(|e| {
                    e.event_type == AuditEventType::UserLogin && e.outcome == AuditOutcome::Failure
                })
                .count();

            (security_incidents, compliance_violations, failed_logins)
        };

        let audit_coverage = {
            let events = self.events.read().await;
            Self::calculate_audit_coverage(&events)
        };

        ComplianceReport {
            generated_at: Utc::now(),
            total_events: stats.total_events,
            security_incidents,
            compliance_violations,
            failed_logins,
            unique_users: stats.unique_users,
            critical_events: stats
                .events_by_severity
                .get(&AuditSeverity::Critical)
                .copied()
                .unwrap_or(0),
            audit_coverage,
        }
    }

    pub(super) fn calculate_audit_coverage(events: &VecDeque<AuditEvent>) -> f64 {
        let expected_types = 12;
        let covered_types = events
            .iter()
            .map(|e| e.event_type.clone())
            .collect::<std::collections::HashSet<_>>()
            .len();

        (f64::from(covered_types as i32) / f64::from(expected_types)) * 100.0
    }
}
