use super::*;

impl Default for InMemoryAuditStore {
    fn default() -> Self {
        Self::new(100_000)
    }
}
impl InMemoryAuditStore {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_events,
        }
    }
}
impl AuditStore for InMemoryAuditStore {
    async fn store(&self, event: AuditEvent) -> Result<()> {
        let mut events = self.events.write().await;

        if events.len() >= self.max_events {
            events.remove(0);
        }

        events.push(event);
        Ok(())
    }

    async fn store_batch(&self, new_events: Vec<AuditEvent>) -> Result<()> {
        let mut events = self.events.write().await;

        for event in new_events {
            if events.len() >= self.max_events {
                events.remove(0);
            }
            events.push(event);
        }

        Ok(())
    }

    async fn query(&self, query: AuditQuery) -> Result<AuditQueryResult> {
        let events = self.events.read().await;

        let filtered: Vec<AuditEvent> = events
            .iter()
            .filter(|e| {
                if let Some(ref start) = query.start_time {
                    if e.timestamp < *start {
                        return false;
                    }
                }
                if let Some(ref end) = query.end_time {
                    if e.timestamp > *end {
                        return false;
                    }
                }
                if let Some(ref types) = query.event_types {
                    if !types.contains(&e.event_type) {
                        return false;
                    }
                }
                if let Some(ref categories) = query.categories {
                    if !categories.contains(&e.category) {
                        return false;
                    }
                }
                if let Some(ref severities) = query.severities {
                    if !severities.contains(&e.severity) {
                        return false;
                    }
                }
                if let Some(ref outcomes) = query.outcomes {
                    if !outcomes.contains(&e.outcome) {
                        return false;
                    }
                }
                if let Some(actor_id) = query.actor_id {
                    if e.actor.user_id != Some(actor_id) {
                        return false;
                    }
                }
                if let Some(ref org_id) = query.organization_id {
                    if e.organization_id.as_ref() != Some(org_id) {
                        return false;
                    }
                }
                if let Some(ref resource_type) = query.resource_type {
                    if let Some(ref resource) = e.resource {
                        if &resource.resource_type != resource_type {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if let Some(ref search) = query.search_text {
                    let search_lower = search.to_lowercase();
                    if !e.description.to_lowercase().contains(&search_lower)
                        && !e.action.to_lowercase().contains(&search_lower)
                    {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total_count = filtered.len();
        let has_more = query.offset + query.limit < total_count;

        let page: Vec<AuditEvent> = filtered
            .into_iter()
            .rev()
            .skip(query.offset)
            .take(query.limit)
            .collect();

        Ok(AuditQueryResult {
            events: page,
            total_count,
            has_more,
        })
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<AuditEvent>> {
        let events = self.events.read().await;
        Ok(events.iter().find(|e| e.id == id).cloned())
    }

    async fn get_chain(&self, start_id: Uuid, count: usize) -> Result<Vec<AuditEvent>> {
        let events = self.events.read().await;

        let start_idx = events.iter().position(|e| e.id == start_id);
        if let Some(idx) = start_idx {
            let end_idx = (idx + count).min(events.len());
            Ok(events[idx..end_idx].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    async fn verify_chain(&self, start_id: Uuid, end_id: Uuid) -> Result<bool> {
        let events = self.events.read().await;

        let start_idx = events.iter().position(|e| e.id == start_id);
        let end_idx = events.iter().position(|e| e.id == end_id);

        match (start_idx, end_idx) {
            (Some(start), Some(end)) if start <= end => {
                let chain = &events[start..=end];

                for i in 1..chain.len() {
                    if chain[i].previous_hash.as_ref() != Some(&chain[i - 1].hash) {
                        return Ok(false);
                    }
                    if !chain[i].verify_hash() {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn cleanup_old_events(&self, before: DateTime<Utc>) -> Result<usize> {
        let mut events = self.events.write().await;
        let initial_count = events.len();
        events.retain(|e| e.timestamp >= before);
        Ok(initial_count - events.len())
    }
}
