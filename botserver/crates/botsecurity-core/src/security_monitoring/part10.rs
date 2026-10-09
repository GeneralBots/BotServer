use super::*;

impl SecurityMonitor {
    pub async fn is_ip_blocked(&self, ip: &str) -> bool {
        let blocked = self.blocked_ips.read().await;
        if let Some(expires) = blocked.get(ip) {
            Utc::now() < *expires
        } else {
            false
        }
    }

    pub async fn get_lockout_info(&self, identifier: &str) -> Option<LockoutRecord> {
        let lockouts = self.lockouts.read().await;
        lockouts.get(identifier).cloned()
    }

    pub async fn unlock(&self, identifier: &str) -> bool {
        let mut lockouts = self.lockouts.write().await;
        lockouts.remove(identifier).is_some()
    }

    pub async fn unblock_ip(&self, ip: &str) -> bool {
        let mut blocked = self.blocked_ips.write().await;
        blocked.remove(ip).is_some()
    }

    pub async fn block_ip(&self, ip: &str, duration: Duration, reason: &str) {
        let mut blocked = self.blocked_ips.write().await;
        blocked.insert(ip.to_string(), Utc::now() + duration);

        let event = SecurityEvent::new(SecurityEventType::IpBlocked)
            .with_ip(ip.to_string())
            .with_detail("reason", serde_json::json!(reason))
            .with_detail("duration_minutes", serde_json::json!(duration.num_minutes()));

        drop(blocked);
        self.record_event(event).await;

        info!("IP {} blocked for {} minutes: {}", ip, duration.num_minutes(), reason);
    }

    async fn create_alert_from_event(&self, event: &SecurityEvent) {
        let alert = SecurityAlert::new(
            event.severity,
            &format!("Security Event: {}", event.event_type.as_str()),
            &format!(
                "{} event detected{}{}",
                event.event_type.as_str(),
                event
                    .user_id
                    .map(|id| format!(" for user {}", id))
                    .unwrap_or_default(),
                event
                    .ip_address
                    .as_ref()
                    .map(|ip| format!(" from IP {}", ip))
                    .unwrap_or_default()
            ),
        )
        .with_event(event.id);

        let alert_with_user = if let Some(uid) = event.user_id {
            alert.with_user(uid)
        } else {
            alert
        };

        let alert_with_ip = if let Some(ref ip) = event.ip_address {
            alert_with_user.with_ip(ip.clone())
        } else {
            alert_with_user
        };

        let mut alerts = self.alerts.write().await;
        alerts.push(alert_with_ip);
    }

    pub async fn get_alerts(&self, unacknowledged_only: bool, limit: usize) -> Vec<SecurityAlert> {
        let alerts = self.alerts.read().await;

        let filtered: Vec<SecurityAlert> = if unacknowledged_only {
            alerts.iter().filter(|a| !a.acknowledged).cloned().collect()
        } else {
            alerts.clone()
        };

        filtered.into_iter().rev().take(limit).collect()
    }

    pub async fn acknowledge_alert(&self, alert_id: Uuid, by: Uuid) -> bool {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledge(by);
            true
        } else {
            false
        }
    }

    pub async fn resolve_alert(&self, alert_id: Uuid) -> bool {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolve();
            true
        } else {
            false
        }
    }

    pub async fn get_user_profile(&self, user_id: Uuid) -> Option<UserSecurityProfile> {
        let profiles = self.user_profiles.read().await;
        profiles.get(&user_id).cloned()
    }

    pub async fn get_recent_events(
        &self,
        event_type: Option<SecurityEventType>,
        user_id: Option<Uuid>,
        limit: usize,
    ) -> Vec<SecurityEvent> {
        let events = self.events.read().await;

        let filtered: Vec<SecurityEvent> = events
            .iter()
            .filter(|e| {
                if let Some(et) = event_type {
                    if e.event_type != et {
                        return false;
                    }
                }
                if let Some(uid) = user_id {
                    if e.user_id != Some(uid) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        filtered.into_iter().rev().take(limit).collect()
    }

    pub async fn cleanup_old_data(&self) -> usize {
        let cutoff = Utc::now() - Duration::hours(self.config.retention_hours as i64);
        let mut total_cleaned = 0;

        {
            let mut events = self.events.write().await;
            let initial = events.len();
            events.retain(|e| e.timestamp > cutoff);
            total_cleaned += initial - events.len();
        }

        {
            let mut attempts = self.login_attempts.write().await;
            for records in attempts.values_mut() {
                let initial = records.len();
                records.retain(|r| r.timestamp > cutoff);
                total_cleaned += initial - records.len();
            }
        }

        {
            let mut lockouts = self.lockouts.write().await;
            let initial = lockouts.len();
            lockouts.retain(|_, l| !l.is_expired());
            total_cleaned += initial - lockouts.len();
        }

        {
            let mut blocked = self.blocked_ips.write().await;
            let initial = blocked.len();
            blocked.retain(|_, expires| Utc::now() < *expires);
            total_cleaned += initial - blocked.len();
        }

        if total_cleaned > 0 {
            info!("Cleaned up {} old security monitoring records", total_cleaned);
        }

        total_cleaned
    }

    pub fn config(&self) -> &SecurityMonitoringConfig {
        &self.config
    }
}
