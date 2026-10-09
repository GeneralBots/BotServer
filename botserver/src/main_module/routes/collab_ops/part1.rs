use super::*;

pub(super) fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    warn!("collab ops error ({}): {msg}", status.as_u16());
    (status, Json(serde_json::json!({ "error": msg })))
}
pub(super) fn normalize_actor_type(actor_type: &str) -> &'static str {
    if actor_type == "llm" {
        "llm"
    } else {
        "human"
    }
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------
