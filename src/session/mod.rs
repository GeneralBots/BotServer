use redis::{AsyncCommands, Client};
use serde_json;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::shared::UserSession;

pub struct SessionManager {
    pub conn: diesel::PgConnection,
    pub redis: Option<Arc<Client>>,
}

impl SessionManager {
    pub fn new(conn: diesel::PgConnection, redis: Option<Arc<Client>>) -> Self {
        Self { conn, redis }
    }

    pub fn get_user_session(
        &mut self,
        user_id: Uuid,
        bot_id: Uuid,
    ) -> Result<Option<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
            })?;
            let cache_key = format!("session:{}:{}", user_id, bot_id);
            let session_json: Option<String> = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(conn.get(&cache_key))
            })?;
            if let Some(json) = session_json {
                if let Ok(session) = serde_json::from_str::<UserSession>(&json) {
                    return Ok(Some(session));
                }
            }
        }

        use crate::shared::models::user_sessions::dsl::*;
        
        let session = user_sessions
            .filter(user_id.eq(user_id))
            .filter(bot_id.eq(bot_id))
            .order_by(updated_at.desc())
            .first::<UserSession>(&mut self.conn)
            .optional()?;

        if let Some(ref session) = session {
            if let Some(redis_client) = &self.redis {
                let mut conn = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
                })?;
                let cache_key = format!("session:{}:{}", user_id, bot_id);
                let session_json = serde_json::to_string(session)?;
                let _: () = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(conn.set_ex(cache_key, session_json, 1800))
                })?;
            }
        }

        Ok(session)
    }

    pub fn create_session(
        &mut self,
        user_id: Uuid,
        bot_id: Uuid,
        title: &str,
    ) -> Result<UserSession, Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::user_sessions;
        use diesel::insert_into;
        
        let session_id = Uuid::new_v4();
        let new_session = (
            user_sessions::id.eq(session_id),
            user_sessions::user_id.eq(user_id),
            user_sessions::bot_id.eq(bot_id),
            user_sessions::title.eq(title),
        );

        let session = insert_into(user_sessions::table)
            .values(&new_session)
            .get_result::<UserSession>(&mut self.conn)?;

        if let Some(redis_client) = &self.redis {
            let mut conn = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
            })?;
            let cache_key = format!("session:{}:{}", user_id, bot_id);
            let session_json = serde_json::to_string(&session)?;
            let _: () = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(conn.set_ex(cache_key, session_json, 1800))
            })?;
        }

        Ok(session)
    }

    pub fn save_message(
        &mut self,
        session_id: Uuid,
        user_id: Uuid,
        role: &str,
        content: &str,
        message_type: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::message_history;
        use diesel::insert_into;
        
        let message_count: i64 = message_history::table
            .filter(message_history::session_id.eq(session_id))
            .count()
            .get_result(&mut self.conn)?;

        let new_message = (
            message_history::session_id.eq(session_id),
            message_history::user_id.eq(user_id),
            message_history::role.eq(role),
            message_history::content_encrypted.eq(content),
            message_history::message_type.eq(message_type),
            message_history::message_index.eq(message_count + 1),
        );

        insert_into(message_history::table)
            .values(&new_message)
            .execute(&mut self.conn)?;

        use crate::shared::models::user_sessions::dsl::*;
        diesel::update(user_sessions.filter(id.eq(session_id)))
            .set(updated_at.eq(diesel::dsl::now))
            .execute(&mut self.conn)?;

        if let Some(redis_client) = &self.redis {
            if let Some(session_info) = user_sessions
                .filter(id.eq(session_id))
                .select((user_id, bot_id))
                .first::<(Uuid, Uuid)>(&mut self.conn)
                .optional()?
            {
                let (session_user_id, session_bot_id) = session_info;
                let mut conn = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
                })?;
                let cache_key = format!("session:{}:{}", session_user_id, session_bot_id);
                let _: () = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(conn.del(cache_key))
                })?;
            }
        }

        Ok(())
    }

    pub fn get_conversation_history(
        &mut self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::message_history::dsl::*;
        
        let messages = message_history
            .filter(session_id.eq(session_id))
            .filter(user_id.eq(user_id))
            .order_by(message_index.asc())
            .select((role, content_encrypted))
            .load::<(String, String)>(&mut self.conn)?;

        Ok(messages)
    }

    pub fn get_user_sessions(
        &mut self,
        user_id: Uuid,
    ) -> Result<Vec<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;
        
        let sessions = user_sessions
            .filter(user_id.eq(user_id))
            .order_by(updated_at.desc())
            .load::<UserSession>(&mut self.conn)?;
        Ok(sessions)
    }

    pub fn update_answer_mode(
        &mut self,
        user_id: &str,
        bot_id: &str,
        mode: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;
        
        let user_uuid = Uuid::parse_str(user_id)?;
        let bot_uuid = Uuid::parse_str(bot_id)?;

        diesel::update(user_sessions.filter(user_id.eq(user_uuid)).filter(bot_id.eq(bot_uuid)))
            .set((
                answer_mode.eq(mode),
                updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut self.conn)?;

        if let Some(redis_client) = &self.redis {
            let mut conn = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
            })?;
            let cache_key = format!("session:{}:{}", user_uuid, bot_uuid);
            let _: () = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(conn.del(cache_key))
            })?;
        }

        Ok(())
    }

    pub fn update_current_tool(
        &mut self,
        user_id: &str,
        bot_id: &str,
        tool_name: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;
        
        let user_uuid = Uuid::parse_str(user_id)?;
        let bot_uuid = Uuid::parse_str(bot_id)?;

        diesel::update(user_sessions.filter(user_id.eq(user_uuid)).filter(bot_id.eq(bot_uuid)))
            .set((
                current_tool.eq(tool_name),
                updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut self.conn)?;

        if let Some(redis_client) = &self.redis {
            let mut conn = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
            })?;
            let cache_key = format!("session:{}:{}", user_uuid, bot_uuid);
            let _: () = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(conn.del(cache_key))
            })?;
        }

        Ok(())
    }

    pub fn get_session_by_id(
        &mut self,
        session_id: Uuid,
    ) -> Result<Option<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
            })?;
            let cache_key = format!("session_by_id:{}", session_id);
            let session_json: Option<String> = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(conn.get(&cache_key))
            })?;
            if let Some(json) = session_json {
                if let Ok(session) = serde_json::from_str::<UserSession>(&json) {
                    return Ok(Some(session));
                }
            }
        }

        use crate::shared::models::user_sessions::dsl::*;
        
        let session = user_sessions
            .filter(id.eq(session_id))
            .first::<UserSession>(&mut self.conn)
            .optional()?;

        if let Some(ref session) = session {
            if let Some(redis_client) = &self.redis {
                let mut conn = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
                })?;
                let cache_key = format!("session_by_id:{}", session_id);
                let session_json = serde_json::to_string(session)?;
                let _: () = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(conn.set_ex(cache_key, session_json, 1800))
                })?;
            }
        }

        Ok(session)
    }

    pub fn cleanup_old_sessions(
        &mut self,
        days_old: i32,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;
        
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days_old as i64);
        let result = diesel::delete(user_sessions.filter(updated_at.lt(cutoff)))
            .execute(&mut self.conn)?;
        Ok(result as u64)
    }

    pub fn set_current_tool(
        &mut self,
        user_id: &str,
        bot_id: &str,
        tool_name: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::models::user_sessions::dsl::*;
        
        let user_uuid = Uuid::parse_str(user_id)?;
        let bot_uuid = Uuid::parse_str(bot_id)?;

        diesel::update(user_sessions.filter(user_id.eq(user_uuid)).filter(bot_id.eq(bot_uuid)))
            .set((
                current_tool.eq(tool_name),
                updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut self.conn)?;

        if let Some(redis_client) = &self.redis {
            let mut conn = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(redis_client.get_multiplexed_async_connection())
            })?;
            let cache_key = format!("session:{}:{}", user_uuid, bot_uuid);
            let _: () = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(conn.del(cache_key))
            })?;
        }

        Ok(())
    }
}
