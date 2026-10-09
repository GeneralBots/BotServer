use super::*;

/// Returns `true` when the session has outlived its TTL and must be evicted.
/// `SessionUserData.created_at` is the epoch-seconds login timestamp; sessions
/// carry no per-session TTL, so the global constant applies uniformly.
///
/// Public so the auth-middleware session-cache rehydrate closure in `main.rs`
/// applies the same rule as [`resolve_token_user`]; without it a persisted
/// token would stay valid forever on the rehydrate path.
pub fn session_expired(user: &SessionUserData) -> bool {
    let now = chrono::Utc::now().timestamp();
    now.saturating_sub(user.created_at) > SESSION_TTL_SECS
}
