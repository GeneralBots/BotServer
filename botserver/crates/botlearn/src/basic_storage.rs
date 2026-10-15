//! Storage behind the learning BASIC keywords (TEACH, STUDY, QUIZ,
//! FLASHCARD, CURRICULUM, LESSON, PROGRESS, REMEMBER FACT).
//!
//! `learn_facts` backs the remembered facts and flashcards; the generated
//! lesson/quiz/curriculum bodies go to `learn_basic_lessons`. That table is
//! deliberately distinct from `learn_lessons` — migration 6.0.5 created the
//! latter with the course shape (`course_id`, `title`, …), so the keyword
//! storage cannot share the name without breaking one of the two users
//! (#1506).

use diesel::sql_types::{BigInt, Nullable, Text, Uuid};
use diesel::QueryableByName;
use diesel::RunQueryDsl;
use serde_json::Value;
use uuid::Uuid as UuidValue;

use botlib::db_pool::DbPool;

/// Ensure the keyword tables exist (idempotent migration fallback).
pub fn ensure_schema(pool: &DbPool) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS learn_facts (
            id UUID PRIMARY KEY,
            bot_id UUID NOT NULL,
            user_id UUID,
            key TEXT NOT NULL,
            value JSONB NOT NULL,
            kind TEXT NOT NULL DEFAULT 'fact',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (bot_id, key)
        )",
    )
    .execute(&mut conn)
    .map_err(|e| format!("learn_facts: {e}"))?;
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS learn_basic_lessons (
            id UUID PRIMARY KEY,
            bot_id UUID NOT NULL,
            user_id UUID,
            topic TEXT NOT NULL,
            kind TEXT NOT NULL,
            content JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(&mut conn)
    .map_err(|e| format!("learn_basic_lessons: {e}"))?;
    Ok(())
}

/// Stores one generated lesson/quiz/curriculum body.
pub fn persist_lesson(
    pool: &DbPool,
    bot_id: UuidValue,
    user_id: Option<UuidValue>,
    topic: &str,
    kind: &str,
    content: &Value,
) -> Result<UuidValue, String> {
    ensure_schema(pool)?;
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let id = UuidValue::new_v4();
    let body = serde_json::to_string(content).map_err(|e| format!("Serialize: {e}"))?;
    diesel::sql_query(
        "INSERT INTO learn_basic_lessons (id, bot_id, user_id, topic, kind, content)
         VALUES ($1, $2, $3, $4, $5, $6::jsonb)
         ON CONFLICT DO NOTHING",
    )
    .bind::<Uuid, _>(id)
    .bind::<Uuid, _>(bot_id)
    .bind::<Nullable<Uuid>, _>(user_id)
    .bind::<Text, _>(topic)
    .bind::<Text, _>(kind)
    .bind::<Text, _>(body)
    .execute(&mut conn)
    .map_err(|e| format!("Insert lesson: {e}"))?;
    Ok(id)
}

/// Inserts or refreshes a fact keyed by `(bot_id, key)`.
pub fn upsert_fact(
    pool: &DbPool,
    bot_id: UuidValue,
    user_id: Option<UuidValue>,
    key: &str,
    value: &Value,
) -> Result<UuidValue, String> {
    ensure_schema(pool)?;
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let id = UuidValue::new_v4();
    let body = serde_json::to_string(value).map_err(|e| format!("Serialize: {e}"))?;
    diesel::sql_query(
        "INSERT INTO learn_facts (id, bot_id, user_id, key, value, kind)
         VALUES ($1, $2, $3, $4, $5::jsonb, 'fact')
         ON CONFLICT (bot_id, key) DO UPDATE
            SET value = EXCLUDED.value,
                user_id = EXCLUDED.user_id,
                created_at = NOW()",
    )
    .bind::<Uuid, _>(id)
    .bind::<Uuid, _>(bot_id)
    .bind::<Nullable<Uuid>, _>(user_id)
    .bind::<Text, _>(key)
    .bind::<Text, _>(body)
    .execute(&mut conn)
    .map_err(|e| format!("Upsert fact: {e}"))?;
    Ok(id)
}

/// Row shape of the `COUNT(*)` queries.
#[derive(QueryableByName, Debug)]
pub struct FactCount {
    #[diesel(sql_type = BigInt)]
    pub count: i64,
}

/// `(facts, lessons)` recorded for one student under one bot.
pub fn progress_for(
    pool: &DbPool,
    bot_id: UuidValue,
    user_id: UuidValue,
) -> Result<(i64, i64), String> {
    ensure_schema(pool)?;
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let facts: Vec<FactCount> = diesel::sql_query(
        "SELECT COUNT(*) AS count FROM learn_facts WHERE bot_id = $1 AND user_id = $2",
    )
    .bind::<Uuid, _>(bot_id)
    .bind::<Uuid, _>(user_id)
    .get_results(&mut conn)
    .map_err(|e| format!("Count facts: {e}"))?;
    let lessons: Vec<FactCount> = diesel::sql_query(
        "SELECT COUNT(*) AS count FROM learn_basic_lessons WHERE bot_id = $1 AND user_id = $2",
    )
    .bind::<Uuid, _>(bot_id)
    .bind::<Uuid, _>(user_id)
    .get_results(&mut conn)
    .map_err(|e| format!("Count lessons: {e}"))?;
    let f = facts.first().map(|r| r.count).unwrap_or(0);
    let l = lessons.first().map(|r| r.count).unwrap_or(0);
    Ok((f, l))
}
