use chrono::Utc;
use diesel::prelude::*;
use diesel::PgConnection;
use log::info;
use redis::Client;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

use crate::shared::models::UserSession;

#[derive(Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub data: String,
}

pub struct SessionManager {
    conn: PgConnection,
    sessions: HashMap<Uuid, SessionData>,
    waiting_for_input: HashSet<Uuid>,
    redis: Option<Arc<Client>>,
}

impl SessionManager {
    pub fn new(conn: PgConnection, redis_client: Option<Arc<Client>>) -> Self {
        info!("Initializing SessionManager");
        SessionManager {
            conn,
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
            let sess = SessionData {
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

    pub fn mark_waiting(&mut self, session_id: Uuid) {
        self.waiting_for_input.insert(session_id);
        info!("Session {} marked as waiting for input", session_id);
    }

    pub fn get_user_session(
        &mut self,
        uid: Uuid,
        bid: Uuid,
    ) -> Result<Option<UserSession>, Box<dyn Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;

        let result = user_sessions
            .filter(user_id.eq(uid))
            .filter(bot_id.eq(bid))
            .order(created_at.desc())
            .first::<UserSession>(&mut self.conn)
            .optional()?;

        Ok(result)
    }

    pub fn create_session(
        &mut self,
        uid: Uuid,
        bid: Uuid,
        session_title: &str,
    ) -> Result<UserSession, Box<dyn Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;

        // Return an existing session if one already matches the user, bot, and title.
        if let Some(existing) = user_sessions
            .filter(user_id.eq(uid))
            .filter(bot_id.eq(bid))
            .filter(title.eq(session_title))
            .first::<UserSession>(&mut self.conn)
            .optional()?
        {
            return Ok(existing);
        }

        let now = Utc::now();

        // Insert the new session and retrieve the full record in one step.
        let inserted: UserSession = diesel::insert_into(user_sessions)
            .values((
                id.eq(Uuid::new_v4()),
                user_id.eq(uid),
                bot_id.eq(bid),
                title.eq(session_title),
                context_data.eq(serde_json::json!({})),
                answer_mode.eq(0),
                current_tool.eq(None::<String>),
                created_at.eq(now),
                updated_at.eq(now),
            ))
            .returning(UserSession::as_returning())
            .get_result(&mut self.conn)?;

        Ok(inserted)
    }

    pub fn save_message(
        &mut self,
        sess_id: Uuid,
        uid: Uuid,
        role_str: &str,
        content: &str,
        msg_type: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use crate::shared::models::message_history::dsl::*;

        let next_index = message_history
            .filter(session_id.eq(sess_id))
            .count()
            .get_result::<i64>(&mut self.conn)?;

        diesel::insert_into(message_history)
            .values((
                id.eq(Uuid::new_v4()),
                session_id.eq(sess_id),
                user_id.eq(uid),
                role.eq(role_str),
                content_encrypted.eq(content),
                message_type.eq(msg_type),
                message_index.eq(next_index),
                created_at.eq(chrono::Utc::now()),
            ))
            .execute(&mut self.conn)?;

        Ok(())
    }

    pub fn get_conversation_history(
        &mut self,
        sess_id: Uuid,
        _uid: Uuid,
    ) -> Result<Vec<(String, String)>, Box<dyn Error + Send + Sync>> {
        use crate::shared::models::message_history::dsl::*;

        let messages = message_history
            .filter(session_id.eq(sess_id))
            .order(message_index.asc())
            .select((role, content_encrypted))
            .load::<(String, String)>(&mut self.conn)?;

        Ok(messages)
    }

    pub fn get_user_sessions(
        &mut self,
        uid: Uuid,
    ) -> Result<Vec<UserSession>, Box<dyn Error + Send + Sync>> {
        use crate::shared::models::user_sessions;

        let sessions = user_sessions::table
            .filter(user_sessions::user_id.eq(uid))
            .order(user_sessions::created_at.desc())
            .load::<UserSession>(&mut self.conn)?;

        Ok(sessions)
    }

    pub fn update_answer_mode(
        &mut self,
        uid: &str,
        bid: &str,
        mode: i32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;

        let user_uuid = Uuid::parse_str(uid)?;
        let bot_uuid = Uuid::parse_str(bid)?;

        diesel::update(
            user_sessions
                .filter(user_id.eq(user_uuid))
                .filter(bot_id.eq(bot_uuid)),
        )
        .set((answer_mode.eq(mode), updated_at.eq(chrono::Utc::now())))
        .execute(&mut self.conn)?;

        Ok(())
    }
}
