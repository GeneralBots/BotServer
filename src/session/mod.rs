use diesel::PgConnection;
use log::info;
use redis::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub data: String,
}

pub struct SessionManager {
    sessions: HashMap<Uuid, UserSession>,
    waiting_for_input: HashSet<Uuid>,
    redis: Option<Arc<Client>>,
}

impl SessionManager {
    pub fn new(_conn: PgConnection, redis_client: Option<Arc<Client>>) -> Self {
        info!("Initializing SessionManager");
        SessionManager {
            sessions: HashMap::new(),
            waiting_for_input: HashSet::new(),
            redis: redis_client,
        }
    }

    pub fn provide_input(
        &mut self,
        session_id: Uuid,
        input: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!(
            "SessionManager.provide_input called for session {}",
            session_id
        );
        if let Some(sess) = self.sessions.get_mut(&session_id) {
            sess.data = input;
        } else {
            let sess = UserSession {
                id: session_id,
                user_id: None,
                data: input,
            };
            self.sessions.insert(session_id, sess);
        }
        self.waiting_for_input.remove(&session_id);
        Ok(())
    }

    pub fn is_waiting_for_input(&self, session_id: &Uuid) -> bool {
        self.waiting_for_input.contains(session_id)
    }

    pub fn create_session(&mut self) -> Uuid {
        let id = Uuid::new_v4();
        let sess = UserSession {
            id,
            user_id: None,
            data: String::new(),
        };
        self.sessions.insert(id, sess);
        info!("Created session {}", id);
        id
    }

    pub fn mark_waiting(&mut self, session_id: Uuid) {
        self.waiting_for_input.insert(session_id);
        info!("Session {} marked as waiting for input", session_id);
    }

    pub fn get_session(&self, session_id: &Uuid) -> Option<UserSession> {
        self.sessions.get(session_id).cloned()
    }

    pub fn list_sessions(&self) -> Vec<UserSession> {
        self.sessions.values().cloned().collect()
    }
}
