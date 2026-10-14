use super::*;

impl IncidentResponseService {
    pub async fn check_sla_breaches(&self) {
        let now = Utc::now();
        let mut incidents = self.incidents.write().await;

        for incident in incidents.values_mut() {
            if incident.status.is_active() && !incident.sla_breach && now > incident.response_due_at {
                incident.sla_breach = true;
                incident.sla_breach_at = Some(now);
                incident.updated_at = now;

                incident.timeline.push(TimelineEntry {
                    id: Uuid::new_v4(),
                    timestamp: now,
                    event_type: TimelineEventType::SlaBreached,
                    description: "SLA response time breached".to_string(),
                    user: None,
                    automated: true,
                    metadata: HashMap::new(),
                });

                log::warn!(
                    "SLA breach for incident {}: {}",
                    incident.incident_number,
                    incident.title
                );
            }
        }
    }

    pub async fn get_incident(&self, incident_id: Uuid) -> Option<Incident> {
        let incidents = self.incidents.read().await;
        incidents.get(&incident_id).cloned()
    }

    pub async fn list_incidents(
        &self,
        status_filter: Option<IncidentStatus>,
        severity_filter: Option<IncidentSeverity>,
        page: u32,
        per_page: u32,
    ) -> IncidentListResponse {
        let incidents = self.incidents.read().await;

        let mut filtered: Vec<_> = incidents
            .values()
            .filter(|i| {
                if let Some(ref status) = status_filter {
                    if &i.status != status {
                        return false;
                    }
                }
                if let Some(ref severity) = severity_filter {
                    if &i.severity != severity {
                        return false;
                    }
                }
                true
            })
            .collect();

        filtered.sort_by_key(|a| std::cmp::Reverse(a.created_at.clone()));

        let total = filtered.len() as u64;
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        let start = ((page - 1) * per_page) as usize;
        let end = (start + per_page as usize).min(filtered.len());

        let summaries: Vec<IncidentSummary> = filtered[start..end]
            .iter()
            .map(|i| IncidentSummary {
                id: i.id,
                incident_number: i.incident_number.clone(),
                title: i.title.clone(),
                severity: i.severity.as_str().to_string(),
                status: i.status.as_str().to_string(),
                category: i.category.as_str().to_string(),
                assigned_to: i.assigned_to.clone(),
                sla_breach: i.sla_breach,
                response_due_at: i.response_due_at,
                created_at: i.created_at,
            })
            .collect();

        IncidentListResponse {
            incidents: summaries,
            total,
            page,
            per_page,
            total_pages,
        }
    }

    pub async fn get_stats(&self) -> IncidentStats {
        let incidents = self.incidents.read().await;

        let mut by_severity: HashMap<String, u64> = HashMap::new();
        let mut by_category: HashMap<String, u64> = HashMap::new();
        let mut by_status: HashMap<String, u64> = HashMap::new();
        let mut total_resolve_time: i64 = 0;
        let mut resolved_count: u64 = 0;
        let mut sla_breaches: u64 = 0;
        let mut open_incidents: u64 = 0;

        for incident in incidents.values() {
            *by_severity.entry(incident.severity.as_str().to_string()).or_insert(0) += 1;
            *by_category.entry(incident.category.as_str().to_string()).or_insert(0) += 1;
            *by_status.entry(incident.status.as_str().to_string()).or_insert(0) += 1;

            if incident.sla_breach {
                sla_breaches += 1;
            }

            if incident.status.is_active() {
                open_incidents += 1;
            }

            if let Some(resolved_at) = incident.resolved_at {
                let duration = resolved_at - incident.created_at;
                total_resolve_time += duration.num_minutes();
                resolved_count += 1;
            }
        }

        let mean_time_to_resolve_hours = if resolved_count > 0 {
            (total_resolve_time as f64 / resolved_count as f64) / 60.0
        } else {
            0.0
        };

        IncidentStats {
            total_incidents: incidents.len() as u64,
            open_incidents,
            resolved_incidents: resolved_count,
            sla_breaches,
            by_severity,
            by_category,
            by_status,
            mean_time_to_resolve_hours,
            mean_time_to_respond_minutes: 0.0,
        }
    }

    pub(crate) async fn trigger_hooks(&self, trigger: HookTrigger, incident: &Incident) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if hook.enabled && hook.trigger == trigger {
                log::info!(
                    "Triggered hook '{}' for incident {}",
                    hook.name,
                    incident.incident_number
                );
            }
        }
    }

    pub async fn register_hook(&self, hook: AutomationHook) {
        let mut hooks = self.hooks.write().await;
        hooks.push(hook);
    }
}
