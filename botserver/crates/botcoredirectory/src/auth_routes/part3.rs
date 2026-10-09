use super::*;

pub fn set_external_token_verifier(f: fn(&str) -> Option<SessionUserData>) {
    let _ = EXTERNAL_TOKEN_VERIFIER.set(f);
}
/// Resolve an arbitrary bearer token to a session user: suite session cache,
/// then persisted sessions, then the wired external verifier (cloud JWT).
pub fn resolve_token_user(token: &str) -> Option<SessionUserData> {
    if token.is_empty() {
        return None;
    }
    if let Some(user) = SESSION_CACHE.try_read().ok().and_then(|c| c.get(token).cloned()) {
        if !session_expired(&user) {
            return Some(user);
        }
    }
    if let Some(user) = session_from_persisted(token) {
        if !session_expired(&user) {
            return Some(user);
        }
    }
    EXTERNAL_TOKEN_VERIFIER
        .get()
        .and_then(|f| f(token))
}
pub fn persist_session(token: &str, user: &SessionUserData) {
    let Some(pool) = SESSION_POOL.get() else {
        return;
    };
    let Ok(user_json) = serde_json::to_value(user) else {
        return;
    };
    if let Ok(mut conn) = pool.get() {
        use diesel::RunQueryDsl;
        diesel::sql_query(
            "INSERT INTO login_sessions (token, user_data) VALUES ($1, $2::jsonb) \
             ON CONFLICT (token) DO UPDATE SET user_data = EXCLUDED.user_data, created_at = NOW()",
        )
        .bind::<diesel::sql_types::Text, _>(token)
        .bind::<diesel::sql_types::Text, _>(&user_json.to_string())
        .execute(&mut conn)
        .unwrap_or_else(|e| {
            log::warn!("Failed to persist login session: {e}");
            0
        });
    }
}
pub fn remove_persisted_session(token: &str) {
    let Some(pool) = SESSION_POOL.get() else {
        return;
    };
    if let Ok(mut conn) = pool.get() {
        use diesel::RunQueryDsl;
        diesel::sql_query("DELETE FROM login_sessions WHERE token = $1")
            .bind::<diesel::sql_types::Text, _>(token)
            .execute(&mut conn)
            .unwrap_or_else(|e| {
                log::warn!("Failed to remove persisted login session: {e}");
                0
            });
    }
}
/// Rehydrates a session from the `login_sessions` table for in-memory cache
/// misses (e.g. after a restart). Returns `None` when no row exists for the
/// token or the stored payload cannot be parsed.
/// #1364 — bridge to the bootstrap-wired external verifier (cloud JWT).
/// Kept as a free function so the handler stays concise and the verifier
/// remains swappable in one place.
pub(super) fn resolve_cloud_bearer(token: &str) -> Option<SessionUserData> {
    EXTERNAL_TOKEN_VERIFIER.get().and_then(|f| f(token))
}
pub fn session_from_persisted(token: &str) -> Option<SessionUserData> {
    let pool = SESSION_POOL.get()?;
    let mut conn = pool.get().ok()?;
    use diesel::RunQueryDsl;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Text)]
        user_data: String,
    }
    // `user_data` is JSONB: PostgreSQL returns binary JSONB with a leading
    // version byte, and the `Text` decoder keeps it, so the payload must be
    // cast to `text` for `serde_json` to parse it (see the same cast in the
    // session-cache rehydrate closure in `main.rs`).
    let row: Row = diesel::sql_query(
        "SELECT user_data::text FROM login_sessions WHERE token = $1 LIMIT 1",
    )
    .bind::<diesel::sql_types::Text, _>(token)
    .get_result(&mut conn)
    .ok()?;
    match serde_json::from_str(&row.user_data) {
        Ok(user) => Some(user),
        Err(e) => {
            // Silently dropping the session here made a broken lookup
            // indistinguishable from an anonymous caller in the logs.
            debug!("session_from_persisted: stored user_data is not valid JSON: {e}");
            None
        }
    }
}
