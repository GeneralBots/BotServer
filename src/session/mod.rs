use redis::{AsyncCommands, Client};
use serde_json;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

use crate::shared::UserSession;

pub struct SessionManager {
    pub pool: PgPool,
    pub redis: Option<Arc<Client>>,
}

impl SessionManager {
    pub fn new(pool: PgPool, redis: Option<Arc<Client>>) -> Self {
        Self { pool, redis }
    }

    pub async fn get_user_session(
        &self,
        user_id: Uuid,
        bot_id: Uuid,
    ) -> Result<Option<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let cache_key = format!("session:{}:{}", user_id, bot_id);
            let session_json: Option<String> = conn.get(&cache_key).await?;
            if let Some(json) = session_json {
                if let Ok(session) = serde_json::from_str::<UserSession>(&json) {
                    return Ok(Some(session));
                }
            }
        }

        let session = sqlx::query_as::<_, UserSession>(
            "SELECT * FROM user_sessions WHERE user_id = $1 AND bot_id = $2 ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(bot_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(ref session) = session {
            if let Some(redis_client) = &self.redis {
                let mut conn = redis_client.get_multiplexed_async_connection().await?;
                let cache_key = format!("session:{}:{}", user_id, bot_id);
                let session_json = serde_json::to_string(session)?;
                let _: () = conn.set_ex(cache_key, session_json, 1800).await?;
            }
        }

        Ok(session)
    }

    pub async fn create_session(
        &self,
        user_id: Uuid,
        bot_id: Uuid,
        title: &str,
    ) -> Result<UserSession, Box<dyn std::error::Error + Send + Sync>> {
        let session = sqlx::query_as::<_, UserSession>(
            "INSERT INTO user_sessions (user_id, bot_id, title) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind(bot_id)
        .bind(title)
        .fetch_one(&self.pool)
        .await?;

        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let cache_key = format!("session:{}:{}", user_id, bot_id);
            let session_json = serde_json::to_string(&session)?;
            let _: () = conn.set_ex(cache_key, session_json, 1800).await?;
        }

        Ok(session)
    }

    pub async fn save_message(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        role: &str,
        content: &str,
        message_type: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message_count: i64 =
            sqlx::query("SELECT COUNT(*) as count FROM message_history WHERE session_id = $1")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await?
                .get("count");

        sqlx::query(
            "INSERT INTO message_history (session_id, user_id, role, content_encrypted, message_type, message_index)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(session_id)
        .bind(user_id)
        .bind(role)
        .bind(content)
        .bind(message_type)
        .bind(message_count + 1)
        .execute(&self.pool)
        .await?;

        sqlx::query("UPDATE user_sessions SET updated_at = NOW() WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        if let Some(redis_client) = &self.redis {
            if let Some(session_info) =
                sqlx::query("SELECT user_id, bot_id FROM user_sessions WHERE id = $1")
                    .bind(session_id)
                    .fetch_optional(&self.pool)
                    .await?
            {
                let user_id: Uuid = session_info.get("user_id");
                let bot_id: Uuid = session_info.get("bot_id");
                let mut conn = redis_client.get_multiplexed_async_connection().await?;
                let cache_key = format!("session:{}:{}", user_id, bot_id);
                let _: () = conn.del(cache_key).await?;
            }
        }

        Ok(())
    }

    pub async fn get_conversation_history(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        let messages = sqlx::query(
            "SELECT role, content_encrypted FROM message_history
             WHERE session_id = $1 AND user_id = $2
             ORDER BY message_index ASC",
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let history = messages
            .into_iter()
            .map(|row| (row.get("role"), row.get("content_encrypted")))
            .collect();

        Ok(history)
    }

    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        let sessions = sqlx::query_as::<_, UserSession>(
            "SELECT * FROM user_sessions WHERE user_id = $1 ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }

    pub async fn update_answer_mode(
        &self,
        user_id: &str,
        bot_id: &str,
        mode: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let user_uuid = Uuid::parse_str(user_id)?;
        let bot_uuid = Uuid::parse_str(bot_id)?;

        sqlx::query(
            "UPDATE user_sessions
             SET answer_mode = $1, updated_at = NOW()
             WHERE user_id = $2 AND bot_id = $3",
        )
        .bind(mode)
        .bind(user_uuid)
        .bind(bot_uuid)
        .execute(&self.pool)
        .await?;

        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let cache_key = format!("session:{}:{}", user_uuid, bot_uuid);
            let _: () = conn.del(cache_key).await?;
        }

        Ok(())
    }

    pub async fn update_current_tool(
        &self,
        user_id: &str,
        bot_id: &str,
        tool_name: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let user_uuid = Uuid::parse_str(user_id)?;
        let bot_uuid = Uuid::parse_str(bot_id)?;

        sqlx::query(
            "UPDATE user_sessions
             SET current_tool = $1, updated_at = NOW()
             WHERE user_id = $2 AND bot_id = $3",
        )
        .bind(tool_name)
        .bind(user_uuid)
        .bind(bot_uuid)
        .execute(&self.pool)
        .await?;

        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let cache_key = format!("session:{}:{}", user_uuid, bot_uuid);
            let _: () = conn.del(cache_key).await?;
        }

        Ok(())
    }

    pub async fn get_session_by_id(
        &self,
        session_id: Uuid,
    ) -> Result<Option<UserSession>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let cache_key = format!("session_by_id:{}", session_id);
            let session_json: Option<String> = conn.get(&cache_key).await?;
            if let Some(json) = session_json {
                if let Ok(session) = serde_json::from_str::<UserSession>(&json) {
                    return Ok(Some(session));
                }
            }
        }

        let session = sqlx::query_as::<_, UserSession>("SELECT * FROM user_sessions WHERE id = $1")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(ref session) = session {
            if let Some(redis_client) = &self.redis {
                let mut conn = redis_client.get_multiplexed_async_connection().await?;
                let cache_key = format!("session_by_id:{}", session_id);
                let session_json = serde_json::to_string(session)?;
                let _: () = conn.set_ex(cache_key, session_json, 1800).await?;
            }
        }

        Ok(session)
    }

    pub async fn cleanup_old_sessions(
        &self,
        days_old: i32,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let result = sqlx::query(
            "DELETE FROM user_sessions
             WHERE updated_at < NOW() - INTERVAL '1 day' * $1",
        )
        .bind(days_old)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn set_current_tool(
        &self,
        user_id: &str,
        bot_id: &str,
        tool_name: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let user_uuid = Uuid::parse_str(user_id)?;
        let bot_uuid = Uuid::parse_str(bot_id)?;

        sqlx::query(
            "UPDATE user_sessions
                 SET current_tool = $1, updated_at = NOW()
                 WHERE user_id = $2 AND bot_id = $3",
        )
        .bind(tool_name)
        .bind(user_uuid)
        .bind(bot_uuid)
        .execute(&self.pool)
        .await?;

        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let cache_key = format!("session:{}:{}", user_uuid, bot_uuid);
            let _: () = conn.del(cache_key).await?;
        }

        Ok(())
    }
}

impl Clone for SessionManager {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            redis: self.redis.clone(),
        }
    }
}
