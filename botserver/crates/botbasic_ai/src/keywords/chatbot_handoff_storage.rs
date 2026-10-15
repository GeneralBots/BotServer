use std::sync::Arc;

use botbasic_types::BasicRuntime;
use diesel::sql_types::{BigInt, Nullable, Text, Uuid};
use diesel::QueryableByName;
use serde_json::Value;
use uuid::Uuid as UuidValue;
use diesel::RunQueryDsl;

/// Ensure the handoff_* tables exist (idempotent migration fallback).
pub fn ensure_schema(pool: &botlib::db_pool::DbPool) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS handoff_queue (
            id UUID PRIMARY KEY,
            bot_id UUID NOT NULL,
            session_id UUID,
            user_id UUID,
            topic TEXT NOT NULL,
            reason TEXT,
            priority TEXT NOT NULL DEFAULT 'normal',
            status TEXT NOT NULL DEFAULT 'queued',
            agent_id UUID,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(&mut conn)
    .map_err(|e| format!("handoff_queue: {e}"))?;
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS handoff_events (
            id UUID PRIMARY KEY,
            handoff_id UUID NOT NULL,
            kind TEXT NOT NULL,
            payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            actor_id UUID,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(&mut conn)
    .map_err(|e| format!("handoff_events: {e}"))?;
    Ok(())
}

#[derive(QueryableByName, Debug)]
pub struct QueuePosition {
    #[diesel(sql_type = BigInt)]
    pub position: i64,
}

#[derive(QueryableByName, Debug)]
pub struct AgentId {
    #[diesel(sql_type = Nullable<Uuid>)]
    pub agent_id: Option<UuidValue>,
}

pub fn queue_position(
    pool: &botlib::db_pool::DbPool,
    bot_id: UuidValue,
    handoff_id: UuidValue,
) -> Result<i64, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let rows: Vec<QueuePosition> = diesel::sql_query(
        "SELECT COUNT(*) AS position
           FROM handoff_queue
          WHERE bot_id = $1
            AND status = 'queued'
            AND created_at <= (SELECT created_at FROM handoff_queue WHERE id = $2)",
    )
    .bind::<Uuid, _>(bot_id)
    .bind::<Uuid, _>(handoff_id)
    .get_results(&mut conn)
    .map_err(|e| format!("queue position: {e}"))?;
    Ok(rows.first().map(|r| r.position).unwrap_or(1))
}

/// The handoff queue row to create.
pub struct NewQueueEntry<'a> {
    pub pool: &'a botlib::db_pool::DbPool,
    pub bot_id: UuidValue,
    pub session_id: Option<UuidValue>,
    pub user_id: Option<UuidValue>,
    pub topic: &'a str,
    pub reason: Option<&'a str>,
    pub priority: &'a str,
    pub metadata: &'a Value,
}

pub fn insert_queue_entry(entry: NewQueueEntry<'_>) -> Result<UuidValue, String> {
    let NewQueueEntry {
        pool,
        bot_id,
        session_id,
        user_id,
        topic,
        reason,
        priority,
        metadata,
    } = entry;
    ensure_schema(pool)?;
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let id = UuidValue::new_v4();
    let body = serde_json::to_string(metadata).map_err(|e| format!("Serialize: {e}"))?;
    diesel::sql_query(
        "INSERT INTO handoff_queue
            (id, bot_id, session_id, user_id, topic, reason, priority, status, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'queued', $8::jsonb)",
    )
    .bind::<Uuid, _>(id)
    .bind::<Uuid, _>(bot_id)
    .bind::<Nullable<Uuid>, _>(session_id)
    .bind::<Nullable<Uuid>, _>(user_id)
    .bind::<Text, _>(topic)
    .bind::<Nullable<Text>, _>(reason)
    .bind::<Text, _>(priority)
    .bind::<Text, _>(body)
    .execute(&mut conn)
    .map_err(|e| format!("insert queue: {e}"))?;
    Ok(id)
}

pub fn update_handoff_status(
    pool: &botlib::db_pool::DbPool,
    handoff_id: UuidValue,
    status: &str,
    agent_id: Option<UuidValue>,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    diesel::sql_query(
        "UPDATE handoff_queue
            SET status = $1,
                agent_id = COALESCE($2, agent_id),
                updated_at = NOW()
          WHERE id = $3",
    )
    .bind::<Text, _>(status)
    .bind::<Nullable<Uuid>, _>(agent_id)
    .bind::<Uuid, _>(handoff_id)
    .execute(&mut conn)
    .map_err(|e| format!("update handoff: {e}"))?;
    Ok(())
}

pub fn fetch_handoff(
    pool: &botlib::db_pool::DbPool,
    handoff_id: UuidValue,
) -> Result<Option<UuidValue>, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let rows: Vec<AgentId> = diesel::sql_query("SELECT agent_id FROM handoff_queue WHERE id = $1")
        .bind::<Uuid, _>(handoff_id)
        .get_results(&mut conn)
        .map_err(|e| format!("fetch handoff: {e}"))?;
    Ok(rows.first().and_then(|r| r.agent_id))
}

pub fn append_event(
    pool: &botlib::db_pool::DbPool,
    handoff_id: UuidValue,
    kind: &str,
    payload: &Value,
    actor_id: Option<UuidValue>,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let body = serde_json::to_string(payload).map_err(|e| format!("Serialize: {e}"))?;
    diesel::sql_query(
        "INSERT INTO handoff_events (id, handoff_id, kind, payload, actor_id)
         VALUES ($1, $2, $3, $4::jsonb, $5)",
    )
    .bind::<Uuid, _>(UuidValue::new_v4())
    .bind::<Uuid, _>(handoff_id)
    .bind::<Text, _>(kind)
    .bind::<Text, _>(body)
    .bind::<Nullable<Uuid>, _>(actor_id)
    .execute(&mut conn)
    .map_err(|e| format!("append event: {e}"))?;
    Ok(())
}

pub fn set_agent_presence(state: &Arc<dyn BasicRuntime>, agent_id: UuidValue, online: bool) {
    if let Some(client) = state.cache_client() {
        let key = format!("presence:{agent_id}");
        let value = if online { "online" } else { "offline" };
        let client_clone = Arc::clone(&client);
        let _ = std::thread::Builder::new()
            .name("presence".into())
            .spawn(move || {
                if let Ok(mut conn) = client_clone.get_connection() {
                    use redis::Commands;
                    if online {
                        let _: Result<(), _> = conn.set_ex(&key, value, 300);
                    } else {
                        let _: Result<(), _> = conn.del(&key);
                    }
                }
            })
            .map(|h| h.join().ok())
            .ok();
    }
}
