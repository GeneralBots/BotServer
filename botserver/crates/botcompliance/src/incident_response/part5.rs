use super::*;

/// A new incident as the caller reports it.
pub struct NewIncident {
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub category: IncidentCategory,
    pub affected_systems: Vec<String>,
    pub reported_by: String,
    pub source: IncidentSource,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// One remediation action performed while handling an incident.
pub struct NewAction {
    pub incident_id: Uuid,
    pub action_type: ActionType,
    pub description: String,
    pub performed_by: String,
    pub automated: bool,
    pub success: bool,
    pub result: Option<String>,
}

/// One piece of evidence attached to an incident.
pub struct NewEvidence {
    pub incident_id: Uuid,
    pub evidence_type: EvidenceType,
    pub title: String,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub url: Option<String>,
    pub collected_by: String,
}

impl IncidentResponseService {
    pub fn new() -> Self {
        Self {
            incidents: Arc::new(RwLock::new(HashMap::new())),
            hooks: Arc::new(RwLock::new(Vec::new())),
            escalation_policies: Arc::new(RwLock::new(HashMap::new())),
            runbooks: Arc::new(RwLock::new(HashMap::new())),
            incident_counter: std::sync::atomic::AtomicU64::new(1000),
        }
    }

    pub async fn add_escalation_policy(&self, policy: EscalationPolicy) {
        let mut policies = self.escalation_policies.write().await;
        policies.insert(policy.id, policy);
    }

    pub async fn add_runbook(&self, runbook: Runbook) {
        let mut runbooks = self.runbooks.write().await;
        runbooks.insert(runbook.id, runbook);
    }

    pub(super) fn generate_incident_number(&self) -> String {
        let num = self
            .incident_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let now = Utc::now();
        format!("INC-{}{:02}{:02}-{:04}", now.format("%Y"), now.format("%m"), now.format("%d"), num)
    }

    pub async fn create_incident(&self, new_incident: NewIncident) -> Incident {
        let NewIncident {
            title,
            description,
            severity,
            category,
            affected_systems,
            reported_by,
            source,
            tags,
            metadata,
        } = new_incident;
        let now = Utc::now();
        let incident_id = Uuid::new_v4();
        let incident_number = self.generate_incident_number();
        let response_due_at = now + Duration::minutes(severity.response_time_minutes());

        let initial_timeline = TimelineEntry {
            id: Uuid::new_v4(),
            timestamp: now,
            event_type: TimelineEventType::Created,
            description: format!("Incident created: {title}"),
            user: Some(reported_by.clone()),
            automated: matches!(source, IncidentSource::Automated | IncidentSource::Alert),
            metadata: HashMap::new(),
        };

        let incident = Incident {
            id: incident_id,
            incident_number,
            title,
            description,
            severity,
            status: IncidentStatus::New,
            category,
            affected_systems,
            affected_users_count: None,
            reported_by,
            assigned_to: None,
            team: None,
            source,
            timeline: vec![initial_timeline],
            actions_taken: Vec::new(),
            evidence: Vec::new(),
            related_incidents: Vec::new(),
            tags,
            sla_breach: false,
            sla_breach_at: None,
            response_due_at,
            created_at: now,
            updated_at: now,
            resolved_at: None,
            closed_at: None,
            root_cause: None,
            lessons_learned: None,
            metadata,
        };

        {
            let mut incidents = self.incidents.write().await;
            incidents.insert(incident_id, incident.clone());
        }

        self.trigger_hooks(HookTrigger::IncidentCreated, &incident).await;

        log::info!(
            "Created incident {}: {} (severity: {:?})",
            incident.incident_number,
            incident.title,
            incident.severity
        );

        incident
    }

    pub async fn update_status(
        &self,
        incident_id: Uuid,
        new_status: IncidentStatus,
        user: &str,
    ) -> Result<Incident, String> {
        let mut incidents = self.incidents.write().await;
        let incident = incidents
            .get_mut(&incident_id)
            .ok_or("Incident not found")?;

        let old_status = incident.status.clone();
        incident.status = new_status.clone();
        incident.updated_at = Utc::now();

        if new_status == IncidentStatus::Resolved {
            incident.resolved_at = Some(Utc::now());
        } else if new_status == IncidentStatus::Closed {
            incident.closed_at = Some(Utc::now());
        }

        incident.timeline.push(TimelineEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: TimelineEventType::StatusChanged,
            description: format!("Status changed from {} to {}", old_status.as_str(), new_status.as_str()),
            user: Some(user.to_string()),
            automated: false,
            metadata: HashMap::new(),
        });

        let incident_clone = incident.clone();
        drop(incidents);

        self.trigger_hooks(HookTrigger::StatusChanged, &incident_clone).await;

        Ok(incident_clone)
    }

    pub async fn update_severity(
        &self,
        incident_id: Uuid,
        new_severity: IncidentSeverity,
        user: &str,
    ) -> Result<Incident, String> {
        let mut incidents = self.incidents.write().await;
        let incident = incidents
            .get_mut(&incident_id)
            .ok_or("Incident not found")?;

        let old_severity = incident.severity.clone();
        incident.severity = new_severity.clone();
        incident.updated_at = Utc::now();
        incident.response_due_at = incident.created_at + Duration::minutes(new_severity.response_time_minutes());

        incident.timeline.push(TimelineEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: TimelineEventType::SeverityChanged,
            description: format!("Severity changed from {} to {}", old_severity.as_str(), new_severity.as_str()),
            user: Some(user.to_string()),
            automated: false,
            metadata: HashMap::new(),
        });

        let incident_clone = incident.clone();
        drop(incidents);

        self.trigger_hooks(HookTrigger::SeverityChanged, &incident_clone).await;

        Ok(incident_clone)
    }

    pub async fn assign_incident(
        &self,
        incident_id: Uuid,
        assignee: &str,
        assigner: &str,
    ) -> Result<Incident, String> {
        let mut incidents = self.incidents.write().await;
        let incident = incidents
            .get_mut(&incident_id)
            .ok_or("Incident not found")?;

        incident.assigned_to = Some(assignee.to_string());
        incident.updated_at = Utc::now();

        incident.timeline.push(TimelineEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: TimelineEventType::Assigned,
            description: format!("Assigned to {assignee}"),
            user: Some(assigner.to_string()),
            automated: false,
            metadata: HashMap::new(),
        });

        Ok(incident.clone())
    }

    pub async fn add_action(&self, action: NewAction) -> Result<ActionTaken, String> {
        let NewAction {
            incident_id,
            action_type,
            description,
            performed_by,
            automated,
            success,
            result,
        } = action;
        let mut incidents = self.incidents.write().await;
        let incident = incidents
            .get_mut(&incident_id)
            .ok_or("Incident not found")?;

        let action = ActionTaken {
            id: Uuid::new_v4(),
            action_type,
            description: description.clone(),
            performed_by: performed_by.clone(),
            performed_at: Utc::now(),
            automated,
            success,
            result,
        };

        incident.actions_taken.push(action.clone());
        incident.updated_at = Utc::now();

        incident.timeline.push(TimelineEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: TimelineEventType::ActionTaken,
            description,
            user: Some(performed_by),
            automated,
            metadata: HashMap::new(),
        });

        Ok(action)
    }

    pub async fn add_evidence(&self, evidence: NewEvidence) -> Result<Evidence, String> {
        let NewEvidence {
            incident_id,
            evidence_type,
            title,
            description,
            file_path,
            url,
            collected_by,
        } = evidence;
        let mut incidents = self.incidents.write().await;
        let incident = incidents
            .get_mut(&incident_id)
            .ok_or("Incident not found")?;

        let evidence = Evidence {
            id: Uuid::new_v4(),
            evidence_type,
            title: title.clone(),
            description,
            file_path,
            url,
            hash: None,
            collected_by: collected_by.clone(),
            collected_at: Utc::now(),
            chain_of_custody: vec![CustodyEntry {
                timestamp: Utc::now(),
                action: "Collected".to_string(),
                person: collected_by.clone(),
                notes: None,
            }],
        };

        incident.evidence.push(evidence.clone());
        incident.updated_at = Utc::now();

        incident.timeline.push(TimelineEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: TimelineEventType::EvidenceAdded,
            description: format!("Evidence added: {title}"),
            user: Some(collected_by),
            automated: false,
            metadata: HashMap::new(),
        });

        Ok(evidence)
    }
}
