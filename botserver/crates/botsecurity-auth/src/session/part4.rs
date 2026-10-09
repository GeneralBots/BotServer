use super::*;

impl Session {
    pub fn new(user_id: Uuid, config: &SessionConfig) -> Self {
        let now = Utc::now();
        let idle_duration = Duration::minutes(config.idle_timeout_minutes);
        let absolute_duration = Duration::hours(config.absolute_timeout_hours);

        Self {
            id: generate_session_id(config.session_id_length),
            user_id,
            status: SessionStatus::Active,
            created_at: now,
            last_accessed_at: now,
            expires_at: now + idle_duration,
            absolute_expires_at: now + absolute_duration,
            ip_address: None,
            device_info: None,
            remember_me: false,
            metadata: HashMap::new(),
        }
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_device(mut self, device: DeviceInfo) -> Self {
        self.device_info = Some(device);
        self
    }

    pub fn with_remember_me(mut self, remember: bool) -> Self {
        self.remember_me = remember;
        if remember {
            self.absolute_expires_at = self.created_at + Duration::days(30);
        }
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.status == SessionStatus::Active && !self.is_expired()
    }

    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        now > self.expires_at || now > self.absolute_expires_at
    }

    pub fn touch(&mut self, idle_timeout_minutes: i64) {
        let now = Utc::now();
        self.last_accessed_at = now;
        self.expires_at = now + Duration::minutes(idle_timeout_minutes);
    }

    pub fn revoke(&mut self) {
        self.status = SessionStatus::Revoked;
    }

    pub fn invalidate(&mut self) {
        self.status = SessionStatus::Invalidated;
    }

    pub fn time_until_expiry(&self) -> Duration {
        let idle_remaining = self.expires_at - Utc::now();
        let absolute_remaining = self.absolute_expires_at - Utc::now();

        if idle_remaining < absolute_remaining {
            idle_remaining
        } else {
            absolute_remaining
        }
    }
}
