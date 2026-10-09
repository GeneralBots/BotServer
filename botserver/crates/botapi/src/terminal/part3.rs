use super::*;

impl TerminalManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            sessions: RwLock::new(HashMap::new()),
        })
    }

    pub async fn create_session(&self, session_id: &str) -> Result<TerminalInfo, String> {
        let mut sessions = self.sessions.write().await;

        if sessions.contains_key(session_id) {
            return Err("Session already exists".to_string());
        }

        let mut session = TerminalSession::new(session_id);
        session.start().await?;

        let info = TerminalInfo {
            session_id: session.session_id.clone(),
            container_name: session.container_name.clone(),
            status: "running".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        sessions.insert(session_id.to_string(), Arc::new(Mutex::new(session)));

        Ok(info)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Arc<Mutex<TerminalSession>>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn kill_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            let mut s = session.lock().await;
            s.kill().await?;
        }
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<TerminalInfo> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::new();
        for s in sessions.values() {
            let session = s.lock().await;
            result.push(TerminalInfo {
                session_id: session.session_id.clone(),
                container_name: session.container_name.clone(),
                status: "running".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        result
    }
}
impl Default for TerminalManager {
    fn default() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}
