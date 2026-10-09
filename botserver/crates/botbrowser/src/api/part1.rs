use super::*;

pub fn configure_routes() -> Router<SessionMap> {
    Router::new()
        .route("/api/browser/session", post(create_session))
        .route("/api/browser/session/:id/navigate", post(handle_navigate))
        .route("/api/browser/session/:id/click", post(handle_click))
        .route("/api/browser/session/:id/fill", post(handle_fill))
        .route("/api/browser/session/:id/screenshot", get(handle_screenshot))
        .route("/api/browser/session/:id/extract", get(handle_extract))
        .route("/api/browser/session/:id/execute", post(handle_execute))
        .route("/api/browser/session/:id/state", get(handle_state))
        .route("/api/browser/session/:id/agent", post(handle_agent))
        .route("/api/browser/session/:id/agent/ws", get(handle_agent_ws))
        .route("/api/browser/session/:id/stream", get(handle_stream))
        .route("/api/browser/proxy", get(handle_proxy))
        .route("/api/browser/session/:id", delete(handle_close))
}
