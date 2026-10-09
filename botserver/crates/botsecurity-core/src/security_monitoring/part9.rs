use super::*;

impl SecurityMonitor {
    pub fn new(config: SecurityMonitoringConfig) -> Self {
        Self {
            config,
            events: Arc::new(RwLock::new(Vec::new())),
            login_attempts: Arc::new(RwLock::new(HashMap::new())),
            lockouts: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            user_profiles: Arc::new(RwLock::new(HashMap::new())),
            blocked_ips: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(SecurityMonitoringConfig::default())
    }

    pub async fn record_event(&self, event: SecurityEvent) {
        if !self.config.enabled {
            return;
        }

        let should_alert = match event.severity {
            AlertSeverity::Critical => self.config.alert_on_critical,
            AlertSeverity::High => self.config.alert_on_high,
            _ => false,
        };

        if should_alert {
            self.create_alert_from_event(&event).await;
        }

        let mut events = self.events.write().await;
        events.push(event);

        if events.len() > 100_000 {
            events.remove(0);
        }
    }

    pub async fn record_login_attempt(
        &self,
        user_id: Option<Uuid>,
        ip: &str,
        success: bool,
        user_agent: Option<&str>,
        location: Option<GeoLocation>,
    ) -> Option<SecurityAlert> {
        if !self.config.enabled {
            return None;
        }

        let record = LoginAttemptRecord {
            user_id,
            ip_address: ip.to_string(),
            timestamp: Utc::now(),
            success,
            user_agent: user_agent.map(String::from),
            location: location.clone(),
        };

        let key = user_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| ip.to_string());

        let mut attempts = self.login_attempts.write().await;
        let user_attempts = attempts.entry(key.clone()).or_default();
        user_attempts.push(record);

        let window_start = Utc::now() - Duration::seconds(self.config.brute_force_window_seconds);
        user_attempts.retain(|a| a.timestamp > window_start);

        let failed_count = user_attempts.iter().filter(|a| !a.success).count() as u32;

        drop(attempts);

        let event_type = if success {
            SecurityEventType::LoginSuccess
        } else {
            SecurityEventType::LoginFailure
        };

        let mut event = SecurityEvent::new(event_type).with_ip(ip.to_string());

        if let Some(uid) = user_id {
            event = event.with_user(uid);
        }

        if let Some(ua) = user_agent {
            event = event.with_user_agent(ua.to_string());
        }

        if let Some(loc) = location.clone() {
            event = event.with_location(loc);
        }

        self.record_event(event).await;

        if !success && failed_count >= self.config.brute_force_threshold {
            return self.handle_brute_force(&key, ip, user_id).await;
        }

        if success {
            if let Some(uid) = user_id {
                self.check_login_anomalies(uid, ip, location).await;
            }
        }

        None
    }

    async fn handle_brute_force(
        &self,
        key: &str,
        ip: &str,
        user_id: Option<Uuid>,
    ) -> Option<SecurityAlert> {
        let lockout = LockoutRecord {
            identifier: key.to_string(),
            locked_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(self.config.lockout_duration_minutes),
            reason: "Brute force attack detected".to_string(),
            attempt_count: self.config.brute_force_threshold,
        };

        {
            let mut lockouts = self.lockouts.write().await;
            lockouts.insert(key.to_string(), lockout);
        }

        {
            let mut blocked = self.blocked_ips.write().await;
            blocked.insert(
                ip.to_string(),
                Utc::now() + Duration::minutes(self.config.lockout_duration_minutes),
            );
        }

        if let Some(uid) = user_id {
            let mut profiles = self.user_profiles.write().await;
            let profile = profiles
                .entry(uid)
                .or_insert_with(|| UserSecurityProfile::new(uid));
            profile.lock(Duration::minutes(self.config.lockout_duration_minutes));
        }

        let mut event = SecurityEvent::new(SecurityEventType::BruteForceDetected)
            .with_ip(ip.to_string())
            .with_detail(
                "threshold",
                serde_json::json!(self.config.brute_force_threshold),
            );

        if let Some(uid) = user_id {
            event = event.with_user(uid);
        }

        self.record_event(event.clone()).await;

        warn!(
            "Brute force attack detected for {} from IP {}",
            key, ip
        );

        let alert = SecurityAlert::new(
            AlertSeverity::Critical,
            "Brute Force Attack Detected",
            &format!(
                "Multiple failed login attempts detected for {}. Account locked for {} minutes.",
                key, self.config.lockout_duration_minutes
            ),
        )
        .with_event(event.id)
        .with_ip(ip.to_string());

        let alert_with_user = if let Some(uid) = user_id {
            alert.with_user(uid)
        } else {
            alert
        };

        let mut alerts = self.alerts.write().await;
        alerts.push(alert_with_user.clone());

        Some(alert_with_user)
    }

    async fn check_login_anomalies(
        &self,
        user_id: Uuid,
        ip: &str,
        location: Option<GeoLocation>,
    ) {
        if !self.config.anomaly_detection_enabled {
            return;
        }

        let is_new_ip = {
            let profiles = self.user_profiles.read().await;
            profiles
                .get(&user_id)
                .map(|p| !p.is_known_ip(ip))
                .unwrap_or(true)
        };

        if is_new_ip {
            let event = SecurityEvent::new(SecurityEventType::NewDeviceLogin)
                .with_user(user_id)
                .with_ip(ip.to_string())
                .with_detail("reason", serde_json::json!("new_ip"));

            self.record_event(event).await;

            let mut profiles = self.user_profiles.write().await;
            let profile = profiles
                .entry(user_id)
                .or_insert_with(|| UserSecurityProfile::new(user_id));
            profile.add_known_ip(ip);
        }

        let mut profiles = self.user_profiles.write().await;
        let profile = profiles
            .entry(user_id)
            .or_insert_with(|| UserSecurityProfile::new(user_id));

        if self.config.impossible_travel_detection {
            if let (Some(last_loc), Some(current_loc)) =
                (profile.last_location.as_ref(), location.as_ref())
            {
                if let Some(last_login) = profile.last_login {
                    if let Some(distance) = last_loc.distance_km(current_loc) {
                        let time_diff = (Utc::now() - last_login).num_hours().max(1) as f64;
                        let speed = distance / time_diff;

                        if speed > self.config.max_travel_speed_kmh {
                            let event = SecurityEvent::new(SecurityEventType::ImpossibleTravel)
                                .with_user(user_id)
                                .with_ip(ip.to_string())
                                .with_location(current_loc.clone())
                                .with_detail("distance_km", serde_json::json!(distance))
                                .with_detail("speed_kmh", serde_json::json!(speed));

                            let event_to_record = event;
                            drop(profiles);
                            self.record_event(event_to_record).await;

                            warn!(
                                "Impossible travel detected for user {}: {} km in {} hours",
                                user_id, distance, time_diff
                            );
                            return;
                        }
                    }
                }
            }
        }

        if self.config.geo_anomaly_detection {
            if let Some(current_loc) = location.as_ref() {
                if let Some(ref country) = current_loc.country {
                    let known_countries: Vec<String> = profile
                        .known_locations
                        .iter()
                        .filter_map(|l| l.country.clone())
                        .collect();

                    if !known_countries.is_empty() && !known_countries.contains(country) {
                        let event = SecurityEvent::new(SecurityEventType::GeoAnomalyDetected)
                            .with_user(user_id)
                            .with_ip(ip.to_string())
                            .with_location(current_loc.clone())
                            .with_detail("new_country", serde_json::json!(country));

                        drop(profiles);
                        self.record_event(event).await;
                        return;
                    }
                }
            }
        }

        let profile = profiles
            .entry(user_id)
            .or_insert_with(|| UserSecurityProfile::new(user_id));
        profile.record_login(location);
    }

    pub async fn is_locked(&self, identifier: &str) -> bool {
        let lockouts = self.lockouts.read().await;
        if let Some(lockout) = lockouts.get(identifier) {
            !lockout.is_expired()
        } else {
            false
        }
    }
}
