use super::*;

impl CsrfToken {
    pub fn new(expiry_minutes: i64) -> Self {
        let token = generate_token();
        let now = Utc::now();

        Self {
            token,
            session_id: None,
            created_at: now,
            expires_at: now + Duration::minutes(expiry_minutes),
        }
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.token.is_empty()
    }
}
