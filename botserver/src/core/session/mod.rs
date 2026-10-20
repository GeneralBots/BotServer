pub use botcoresession::*;

pub async fn start_session() {}
pub async fn get_sessions() -> Vec<serde_json::Value> { vec![] }
pub async fn get_session_history() -> Vec<serde_json::Value> { vec![] }
pub async fn create_session() -> String { String::new() }

fn to_lib_session(s: botcoresession::UserSession) -> botlib::models::UserSession {
    botlib::models::UserSession {
        id: s.id,
        user_id: uuid::Uuid::parse_str(&s.user_id).unwrap_or(uuid::Uuid::nil()),
        branch_id: s.branch_id,
        bot_id: s.bot_id,
        title: String::new(),
        context_data: serde_json::Value::Null,
        current_tool: None,
        created_at: s.created_at,
        updated_at: s.updated_at,
    }
}

fn to_lib_session_opt(s: Result<Option<botcoresession::UserSession>, String>) -> Result<Option<botlib::models::UserSession>, String> {
    Ok(s?.map(to_lib_session))
}

fn to_lib_session_res(r: Result<botcoresession::UserSession, String>) -> Result<botlib::models::UserSession, String> {
    r.map(to_lib_session)
}

fn to_lib_session_vec(v: Result<Vec<botcoresession::UserSession>, String>) -> Result<Vec<botlib::models::UserSession>, String> {
    Ok(v?.into_iter().map(to_lib_session).collect())
}

#[derive(Debug)]
pub struct LocalSessionManager(pub botcoresession::SessionManager);

impl botlib::traits::SessionManagerService for LocalSessionManager {
    fn get_session_by_id(&mut self, session_id: uuid::Uuid) -> Result<Option<botlib::models::UserSession>, String> {
        to_lib_session_opt(self.0.get_session_by_id(session_id).map_err(|e| e.to_string()))
    }
    fn get_or_create_user_session(&mut self, user_id: uuid::Uuid, bot_id: uuid::Uuid, session_title: &str) -> Result<Option<botlib::models::UserSession>, String> {
        to_lib_session_opt(self.0.get_or_create_user_session(user_id, bot_id, session_title).map_err(|e| e.to_string()))
    }
    fn get_or_create_anonymous_user(&mut self, user_id: Option<uuid::Uuid>) -> Result<uuid::Uuid, String> {
        self.0.get_or_create_anonymous_user(user_id).map_err(|e| e.to_string())
    }
    fn create_session(&mut self, user_id: uuid::Uuid, bot_id: uuid::Uuid, session_title: &str) -> Result<botlib::models::UserSession, String> {
        to_lib_session_res(self.0.create_session(user_id, bot_id, session_title).map_err(|e| e.to_string()))
    }
    fn get_or_create_session_by_id(&mut self, session_id: uuid::Uuid, user_id: uuid::Uuid, bot_id: uuid::Uuid, session_title: &str) -> Result<botlib::models::UserSession, String> {
        to_lib_session_res(self.0.get_or_create_session_by_id(session_id, user_id, bot_id, session_title).map_err(|e| e.to_string()))
    }
    fn save_message(&mut self, session_id: uuid::Uuid, user_id: uuid::Uuid, role: i32, content: &str, message_type: i32) -> Result<(), String> {
        self.0.save_message(session_id, user_id, role, content, message_type).map_err(|e| e.to_string())
    }
    fn get_conversation_history(&mut self, session_id: uuid::Uuid, user_id: uuid::Uuid, limit: Option<i64>) -> Result<Vec<(String, String)>, String> {
        self.0.get_conversation_history(session_id, user_id, limit).map_err(|e| e.to_string())
    }
    fn get_first_user_message(&mut self, session_id: uuid::Uuid) -> Result<Option<String>, String> {
        self.0.get_first_user_message(session_id).map_err(|e| e.to_string())
    }
    fn get_session_context_data(&self, session_id: &uuid::Uuid, user_id: &uuid::Uuid) -> Result<String, String> {
        self.0.get_session_context_data(session_id, user_id).map(|v| v.to_string()).map_err(|e| e.to_string())
    }
    fn update_session_context(&mut self, session_id: &uuid::Uuid, user_id: &uuid::Uuid, context_data: String) -> Result<(), String> {
        self.0.update_session_context(session_id, user_id, context_data).map_err(|e| e.to_string())
    }
    fn get_user_sessions(&mut self, user_id: uuid::Uuid) -> Result<Vec<botlib::models::UserSession>, String> {
        to_lib_session_vec(self.0.get_user_sessions(user_id).map_err(|e| e.to_string()))
    }
    fn update_user_id(&mut self, session_id: uuid::Uuid, new_user_id: uuid::Uuid) -> Result<(), String> {
        self.0.update_user_id(session_id, new_user_id).map_err(|e| e.to_string())
    }
    fn mark_waiting(&mut self, session_id: uuid::Uuid) {
        self.0.mark_waiting(session_id)
    }
    fn active_count(&self) -> usize {
        self.0.active_count()
    }
}