use axum::http::HeaderMap;
use diesel::prelude::*;
use uuid::Uuid;

use crate::state::TasksState;

pub fn get_user_id_from_headers(
    state: &TasksState,
    headers: &HeaderMap,
) -> Result<Uuid, String> {
    let session_val = headers
        .get("x-session-id")
        .or_else(|| headers.get("cookie"))
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "No session header found".to_string())?;

    let session_val = session_val
        .split(';')
        .find(|s| s.trim().starts_with("session_id="))
        .map(|s| s.trim().strip_prefix("session_id=").unwrap_or(s).trim())
        .unwrap_or(session_val);

    let sid = session_val
        .parse::<Uuid>()
        .map_err(|_| "Invalid session ID format".to_string())?;

    use crate::schema::user_sessions::dsl::*;

    let mut conn = state
        .pool
        .get()
        .map_err(|e| format!("Pool error: {}", e))?;

    let user_id_str: String = user_sessions
        .find(sid)
        .select(user_id)
        .first::<String>(&mut conn)
        .map_err(|e| format!("Session lookup error: {}", e))?;

    user_id_str
        .parse::<Uuid>()
        .map_err(|_| "Invalid user ID format".to_string())
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn truncate(s: &str, max_len: usize) -> String {
    botlib::text::truncate_with_suffix(s, max_len, "...")
}
