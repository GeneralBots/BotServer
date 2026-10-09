use super::*;

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}
impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
impl SessionStore for InMemorySessionStore {
    async fn create(&self, session: Session) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session);
        Ok(())
    }

    async fn get(&self, session_id: &str) -> Result<Option<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    async fn update(&self, session: &Session) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if sessions.contains_key(&session.id) {
            sessions.insert(session.id.clone(), session.clone());
            Ok(())
        } else {
            Err(anyhow!("Session not found: {}", session.id))
        }
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        Ok(())
    }

    async fn get_user_sessions(&self, user_id: Uuid) -> Result<Vec<Session>> {
        let sessions = self.sessions.read().await;
        let user_sessions: Vec<Session> = sessions
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect();
        Ok(user_sessions)
    }

    async fn delete_user_sessions(&self, user_id: Uuid) -> Result<usize> {
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();
        sessions.retain(|_, s| s.user_id != user_id);
        let deleted = initial_count - sessions.len();
        Ok(deleted)
    }

    async fn cleanup_expired(&self) -> Result<usize> {
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();
        sessions.retain(|_, s| !s.is_expired());
        let cleaned = initial_count - sessions.len();
        Ok(cleaned)
    }
}
