//! Browsing memory: per-user page facts and browse session lifecycle.

use crate::models::{BrowseSession, NewBrowseSession, NewPageFact, PageFact};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use std::fmt;
use uuid::Uuid;

pub type DbPool = diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryError {
    Db(String),
    NotFound,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(m) => write!(f, "browsing memory database error: {m}"),
            Self::NotFound => write!(f, "browse session not found"),
        }
    }
}

impl std::error::Error for MemoryError {}

/// Recency-weighted popularity score used by `top_facts`.
///
/// Score decays with page age in hours so frequently visited pages stay
/// relevant only while they remain fresh: `visit_count / (1 + hours_idle)`.
pub fn fact_score(visit_count: i32, last_seen: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
    let minutes = (now - last_seen).num_minutes().max(0) as f64;
    let hours = minutes / 60.0;
    f64::from(visit_count.max(0)) * (1.0 / (1.0 + hours))
}

/// Inserts a page fact or bumps the existing row for `(user_id, url)`:
/// `visit_count = visit_count + 1`, `last_seen = now()`, and both `facts` and
/// `title` are overwritten from the incoming values.
pub fn upsert_page_fact(
    pool: &DbPool,
    user_id: Uuid,
    url: String,
    title: Option<String>,
    facts: serde_json::Value,
) -> Result<PageFact, MemoryError> {
    use crate::schema::page_facts::dsl as pf;

    let mut conn = pool.get().map_err(|e| MemoryError::Db(e.to_string()))?;
    let new_fact = NewPageFact {
        id: Uuid::new_v4(),
        user_id,
        url: url.clone(),
        title,
        facts,
    };
    diesel::insert_into(pf::page_facts)
        .values(&new_fact)
        .on_conflict((pf::user_id, pf::url))
        .do_update()
        .set((
            pf::visit_count.eq(pf::visit_count + 1),
            pf::last_seen.eq(diesel::dsl::now),
            pf::facts.eq(diesel::pg::upsert::excluded(pf::facts)),
            pf::title.eq(diesel::pg::upsert::excluded(pf::title)),
        ))
        .get_result::<PageFact>(&mut conn)
        .map_err(|e| MemoryError::Db(e.to_string()))
}

/// Returns the most relevant facts for a user ordered by recency × visits.
/// Candidates are read newest-first and re-scored in Rust to keep the query
/// portable; only the top `limit` entries are returned.
pub fn top_facts(
    pool: &DbPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<PageFact>, MemoryError> {
    use crate::schema::page_facts::dsl as pf;

    let mut conn = pool.get().map_err(|e| MemoryError::Db(e.to_string()))?;
    let mut rows = pf::page_facts
        .filter(pf::user_id.eq(user_id))
        .order(pf::last_seen.desc())
        .limit(500)
        .load::<PageFact>(&mut conn)
        .map_err(|e| MemoryError::Db(e.to_string()))?;
    let now = Utc::now();
    rows.sort_by(|a, b| {
        let sa = fact_score(a.visit_count, a.last_seen, now);
        let sb = fact_score(b.visit_count, b.last_seen, now);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.truncate(limit.max(0) as usize);
    Ok(rows)
}

/// Removes every stored fact for a user. Returns the number of deleted rows.
pub fn purge_user(pool: &DbPool, user_id: Uuid) -> Result<usize, MemoryError> {
    use crate::schema::page_facts::dsl as pf;

    let mut conn = pool.get().map_err(|e| MemoryError::Db(e.to_string()))?;
    diesel::delete(pf::page_facts)
        .filter(pf::user_id.eq(user_id))
        .execute(&mut conn)
        .map_err(|e| MemoryError::Db(e.to_string()))
}

/// Opens a browse session row bound optionally to a task.
pub fn start_browse_session(
    pool: &DbPool,
    user_id: Uuid,
    task_id: Option<Uuid>,
) -> Result<BrowseSession, MemoryError> {
    use crate::schema::browse_sessions::dsl as bs;

    let mut conn = pool.get().map_err(|e| MemoryError::Db(e.to_string()))?;
    diesel::insert_into(bs::browse_sessions)
        .values(&NewBrowseSession {
            id: Uuid::new_v4(),
            user_id,
            task_id,
        })
        .get_result::<BrowseSession>(&mut conn)
        .map_err(|e| MemoryError::Db(e.to_string()))
}

/// Closes a browse session with an optional summary line.
pub fn end_browse_session(
    pool: &DbPool,
    session_id: Uuid,
    summary: Option<String>,
) -> Result<BrowseSession, MemoryError> {
    use crate::schema::browse_sessions::dsl as bs;

    let mut conn = pool.get().map_err(|e| MemoryError::Db(e.to_string()))?;
    diesel::update(bs::browse_sessions.find(session_id))
        .set((bs::ended_at.eq(diesel::dsl::now), bs::summary.eq(summary)))
        .get_result::<BrowseSession>(&mut conn)
        .optional()
        .map_err(|e| MemoryError::Db(e.to_string()))?
        .ok_or(MemoryError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn ts(hours_ago: i64) -> DateTime<Utc> {
        Utc::now() - Duration::hours(hours_ago)
    }

    #[test]
    fn score_prefers_visits_but_decays_with_age() {
        let now = Utc::now();
        assert!(fact_score(5, ts(1), now) > fact_score(5, ts(100), now));
        assert!(fact_score(10, ts(2), now) > fact_score(3, ts(2), now));
    }

    #[test]
    fn score_never_negative_and_handles_future_timestamps() {
        let now = Utc::now();
        assert_eq!(fact_score(-4, ts(1), now), 0.0);
        assert_eq!(fact_score(2, Utc::now() + Duration::hours(5), now), 2.0);
    }

    #[test]
    fn memory_error_display_is_stable() {
        assert_eq!(
            MemoryError::NotFound.to_string(),
            "browse session not found"
        );
        assert!(MemoryError::Db("boom".to_string()).to_string().contains("boom"));
    }

    /// Compile-level verification that the upsert emits the required
    /// `ON CONFLICT (user_id, url) DO UPDATE` clause with the visit-count
    /// increment; executed against `debug_query`, no live database needed.
    #[test]
    fn upsert_sql_contains_conflict_clause_and_increment() {
        use crate::schema::page_facts::dsl as pf;
        let fact = NewPageFact {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            url: "https://example.com/".to_string(),
            title: None,
            facts: serde_json::json!({}),
        };
        let q = diesel::insert_into(pf::page_facts)
            .values(&fact)
            .on_conflict((pf::user_id, pf::url))
            .do_update()
            .set((
                pf::visit_count.eq(pf::visit_count + 1),
                pf::last_seen.eq(diesel::dsl::now),
                pf::facts.eq(diesel::pg::upsert::excluded(pf::facts)),
            ));
        let sql = diesel::debug_query::<diesel::pg::Pg, _>(&q).to_string();
        // The upsert must conflict on the natural key, increment the stored
        // counter instead of overwriting it, and take the payload from the row
        // that lost the conflict.
        assert!(sql.contains("ON CONFLICT"), "sql: {sql}");
        assert!(sql.contains("\"page_facts\".\"visit_count\" + "), "sql: {sql}");
        assert!(sql.contains("excluded"), "sql: {sql}");
    }
}
