use super::*;

pub(super) fn db_err(e: impl std::fmt::Display) -> (StatusCode, Json<serde_json::Value>) {
    log::error!("Player playlist error: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botplayer::playlists::part1", None) })),
    )
}
