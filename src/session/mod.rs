use crate::shared::models::UserSession;
use chrono::Utc;
use diesel::prelude::*;
use diesel::PgConnection;
use log::{debug, error, info, warn};
use redis::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

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
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!(
            "SessionManager.provide_input called for session {}",
            session_id
        );
        if let Some(sess) = self.sessions.get_mut(&session_id) {
            sess.data = input;
            self.waiting_for_input.remove(&session_id);
            Ok(Some("user_input".to_string()))
        } else {
            let sess = SessionData {
                id: session_id,
                user_id: None,
                data: input,
            };
            self.sessions.insert(session_id, sess);
            self.waiting_for_input.remove(&session_id);
            Ok(Some("user_input".to_string()))
        }
    }

    pub fn is_waiting_for_input(&self, session_id: &Uuid) -> bool {
        self.waiting_for_input.contains(session_id)
    }

    pub fn mark_waiting(&mut self, session_id: Uuid) {
        self.waiting_for_input.insert(session_id);
    }

    pub fn get_session_by_id(
        &mut self,
        session_id: Uuid,
    ) -> Result<Option<UserSession>, Box<dyn Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;
        let result = user_sessions
            .filter(id.eq(session_id))
            .first::<UserSession>(&mut self.conn)
            .optional()?;
        Ok(result)
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

    pub fn get_or_create_user_session(
        &mut self,
        uid: Uuid,
        bid: Uuid,
        session_title: &str,
    ) -> Result<Option<UserSession>, Box<dyn Error + Send + Sync>> {
        if let Some(existing) = self.get_user_session(uid, bid)? {
            return Ok(Some(existing));
        }
        self.create_session(uid, bid, session_title).map(Some)
    }

    pub fn create_session(
        &mut self,
        uid: Uuid,
        bid: Uuid,
        session_title: &str,
    ) -> Result<UserSession, Box<dyn Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;
        use crate::shared::models::users::dsl as users_dsl;

        let now = Utc::now();
        let user_exists: Option<Uuid> = users_dsl::users
            .filter(users_dsl::id.eq(uid))
            .select(users_dsl::id)
            .first(&mut self.conn)
            .optional()?;

        if user_exists.is_none() {
            warn!(
                "User {} does not exist in database, creating placeholder user",
                uid
            );
            diesel::insert_into(users_dsl::users)
                .values((
                    users_dsl::id.eq(uid),
                    users_dsl::username.eq(format!("anonymous_{}", rand::random::<u32>())),
                    users_dsl::email.eq(format!("anonymous_{}@local", rand::random::<u32>())),
                    users_dsl::password_hash.eq("placeholder"),
                    users_dsl::is_active.eq(true),
                    users_dsl::created_at.eq(now),
                    users_dsl::updated_at.eq(now),
                ))
                .execute(&mut self.conn)?;
        }

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
            .get_result(&mut self.conn)
            .map_err(|e| {
                error!("Failed to create session in database: {}", e);
                e
            })?;

        Ok(inserted)
    }

    pub fn save_message(
        &mut self,
        sess_id: Uuid,
        uid: Uuid,
        ro: i32,
        content: &str,
        msg_type: i32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use crate::shared::models::message_history::dsl::*;

        let next_index = message_history
            .filter(session_id.eq(sess_id))
            .count()
            .get_result::<i64>(&mut self.conn)
            .unwrap_or(0);

        diesel::insert_into(message_history)
            .values((
                id.eq(Uuid::new_v4()),
                session_id.eq(sess_id),
                user_id.eq(uid),
                role.eq(ro),
                content_encrypted.eq(content),
                message_type.eq(msg_type),
                message_index.eq(next_index),
                created_at.eq(chrono::Utc::now()),
            ))
            .execute(&mut self.conn)?;

        debug!(
            "Message saved for session {} with index {}",
            sess_id, next_index
        );
        Ok(())
    }

    pub async fn get_session_context(
        &self,
        session_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Bring the Redis command trait into scope so we can call `get`.
        use redis::Commands;

        let redis_key = format!("context:{}:{}", user_id, session_id);
        if let Some(redis_client) = &self.redis {
            // Attempt to obtain a Redis connection; log and ignore errors, returning `None`.
            let conn_option = redis_client
                .get_connection()
                .map_err(|e| {
                    warn!("Failed to get Redis connection: {}", e);
                    e
                })
                .ok();

            if let Some(mut connection) = conn_option {
                match connection.get::<_, Option<String>>(&redis_key) {
                    Ok(Some(context)) => {
                        debug!(
                            "Retrieved context from Redis for key {}: {} chars",
                            redis_key,
                            context.len()
                        );
                        return Ok(context);
                    }
                    Ok(None) => {
                        debug!("No context found in Redis for key {}", redis_key);
                    }
                    Err(e) => {
                        warn!("Failed to retrieve context from Redis: {}", e);
                    }
                }
            }
        }
        // If Redis is unavailable or the key is missing, return an empty context.
        Ok(String::new())
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
            .load::<(i32, String)>(&mut self.conn)?;

        let mut history: Vec<(String, String)> = Vec::new();
        for (other_role, content) in messages {
            let role_str = match other_role {
                0 => "user".to_string(),
                1 => "assistant".to_string(),
                2 => "system".to_string(),
                _ => "unknown".to_string(),
            };
            history.push((role_str, content));
        }
        Ok(history)
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

        let user_uuid = Uuid::parse_str(uid).map_err(|e| {
            warn!("Invalid user ID format: {}", uid);
            e
        })?;
        let bot_uuid = Uuid::parse_str(bid).map_err(|e| {
            warn!("Invalid bot ID format: {}", bid);
            e
        })?;

        let updated_count = diesel::update(
            user_sessions
                .filter(user_id.eq(user_uuid))
                .filter(bot_id.eq(bot_uuid)),
        )
        .set((answer_mode.eq(mode), updated_at.eq(chrono::Utc::now())))
        .execute(&mut self.conn)?;

        if updated_count == 0 {
            warn!("No session found for user {} and bot {}", uid, bid);
        } else {
            info!(
                "Answer mode updated to {} for user {} and bot {}",
                mode, uid, bid
            );
        }
        Ok(())
    }

    pub fn update_user_id(
        &mut self,
        session_id: Uuid,
        new_user_id: Uuid,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;

        let updated_count = diesel::update(user_sessions.filter(id.eq(session_id)))
            .set((user_id.eq(new_user_id), updated_at.eq(chrono::Utc::now())))
            .execute(&mut self.conn)?;

        if updated_count == 0 {
            warn!("No session found with ID: {}", session_id);
        } else {
            debug!("Updated user ID for session {}", session_id);
        }
        Ok(())
    }
}
