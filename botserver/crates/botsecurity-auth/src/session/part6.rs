use super::*;

impl<S: SessionStore> SessionManager<S> {
    pub fn new(store: S, config: SessionConfig) -> Self {
        Self { store, config }
    }

    pub async fn create_session(
        &self,
        user_id: Uuid,
        ip_address: Option<String>,
        user_agent: Option<&str>,
        remember_me: bool,
    ) -> Result<Session> {
        let existing_sessions = self.store.get_user_sessions(user_id).await?;
        let active_count = existing_sessions.iter().filter(|s| s.is_valid()).count();

        if active_count >= self.config.max_concurrent_sessions {
            let mut oldest_sessions: Vec<_> = existing_sessions
                .into_iter()
                .filter(|s| s.is_valid())
                .collect();
            oldest_sessions.sort_by_key(|s| s.last_accessed_at);

            let sessions_to_remove = active_count - self.config.max_concurrent_sessions + 1;
            for session in oldest_sessions.iter().take(sessions_to_remove) {
                self.store.delete(&session.id).await?;
                debug!("Removed oldest session {} for user {user_id}", session.id);
            }
        }

        let mut session = Session::new(user_id, &self.config).with_remember_me(remember_me);

        if let Some(ip) = ip_address {
            session = session.with_ip(ip);
        }

        if self.config.enable_device_tracking {
            if let Some(ua) = user_agent {
                session = session.with_device(DeviceInfo::from_user_agent(ua));
            }
        }

        self.store.create(session.clone()).await?;
        info!("Created session {} for user {user_id}", session.id);

        Ok(session)
    }

    pub async fn validate_session(&self, session_id: &str) -> Result<Option<Session>> {
        let session = match self.store.get(session_id).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        if !session.is_valid() {
            if session.is_expired() {
                self.store.delete(session_id).await?;
                debug!("Cleaned up expired session {session_id}");
            }
            return Ok(None);
        }

        Ok(Some(session))
    }

    pub async fn touch_session(&self, session_id: &str) -> Result<bool> {
        let mut session = match self.store.get(session_id).await? {
            Some(s) if s.is_valid() => s,
            _ => return Ok(false),
        };

        session.touch(self.config.idle_timeout_minutes);
        self.store.update(&session).await?;

        Ok(true)
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<bool> {
        let mut session = match self.store.get(session_id).await? {
            Some(s) => s,
            None => return Ok(false),
        };

        session.revoke();
        self.store.update(&session).await?;
        info!("Revoked session {session_id}");

        Ok(true)
    }

    pub async fn revoke_all_user_sessions(&self, user_id: Uuid) -> Result<usize> {
        let sessions = self.store.get_user_sessions(user_id).await?;
        let mut revoked = 0;

        for mut session in sessions {
            if session.status == SessionStatus::Active {
                session.revoke();
                self.store.update(&session).await?;
                revoked += 1;
            }
        }

        info!("Revoked {revoked} sessions for user {user_id}");
        Ok(revoked)
    }

    pub async fn revoke_all_except(&self, user_id: Uuid, keep_session_id: &str) -> Result<usize> {
        let sessions = self.store.get_user_sessions(user_id).await?;
        let mut revoked = 0;

        for mut session in sessions {
            if session.id != keep_session_id && session.status == SessionStatus::Active {
                session.revoke();
                self.store.update(&session).await?;
                revoked += 1;
            }
        }

        info!("Revoked {revoked} other sessions for user {user_id}");
        Ok(revoked)
    }

    pub async fn get_user_sessions(&self, user_id: Uuid) -> Result<Vec<Session>> {
        let sessions = self.store.get_user_sessions(user_id).await?;
        Ok(sessions.into_iter().filter(|s| s.is_valid()).collect())
    }

    pub async fn regenerate_session(&self, old_session_id: &str, ip_address: Option<String>, user_agent: Option<&str>) -> Result<Option<Session>> {
        let old_session = match self.store.get(old_session_id).await? {
            Some(s) if s.is_valid() => s,
            _ => return Ok(None),
        };

        let user_id = old_session.user_id;

        let mut new_session = Session::new(user_id, &self.config)
            .with_remember_me(old_session.remember_me)
            .with_metadata("regenerated_from".to_string(), old_session.id.clone());

        if let Some(ip) = ip_address {
            new_session = new_session.with_ip(ip);
        }

        if self.config.enable_device_tracking {
            if let Some(ua) = user_agent {
                new_session = new_session.with_device(DeviceInfo::from_user_agent(ua));
            }
        }

        for (key, value) in old_session.metadata {
            if key != "regenerated_from" {
                new_session = new_session.with_metadata(key, value);
            }
        }

        self.store.delete(old_session_id).await?;
        self.store.create(new_session.clone()).await?;

        info!("Regenerated session {} -> {} for user {user_id}", old_session_id, new_session.id);

        Ok(Some(new_session))
    }

    pub async fn invalidate_on_password_change(&self, user_id: Uuid) -> Result<usize> {
        let count = self.store.delete_user_sessions(user_id).await?;
        info!("Invalidated {count} sessions for user {user_id} due to password change");
        Ok(count)
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        let cleaned = self.store.cleanup_expired().await?;
        if cleaned > 0 {
            info!("Cleaned up {cleaned} expired sessions");
        }
        Ok(cleaned)
    }

    pub fn build_cookie(&self, session: &Session) -> String {
        let max_age = session.time_until_expiry().num_seconds().max(0);
        let secure = if self.config.cookie_secure {
            "; Secure"
        } else {
            ""
        };
        let http_only = if self.config.cookie_http_only {
            "; HttpOnly"
        } else {
            ""
        };
        let same_site = format!("; SameSite={}", self.config.cookie_same_site.as_str());

        format!(
            "{}={}; Path=/; Max-Age={max_age}{secure}{http_only}{same_site}",
            self.config.cookie_name, session.id
        )
    }

    pub fn build_logout_cookie(&self) -> String {
        let secure = if self.config.cookie_secure {
            "; Secure"
        } else {
            ""
        };
        let http_only = if self.config.cookie_http_only {
            "; HttpOnly"
        } else {
            ""
        };
        let same_site = format!("; SameSite={}", self.config.cookie_same_site.as_str());

        format!(
            "{}=; Path=/; Max-Age=0{secure}{http_only}{same_site}",
            self.config.cookie_name
        )
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
}
pub fn generate_session_id(length: usize) -> String {
    use rand::Rng;

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();

    (0..length)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}
pub fn extract_session_id_from_cookie(cookie_header: &str, cookie_name: &str) -> Option<String> {
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=') {
            if name.trim() == cookie_name {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}
