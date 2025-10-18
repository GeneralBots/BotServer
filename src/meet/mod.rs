use actix_web::{web, HttpResponse, Result};
use log::{error, info};

use crate::shared::state::AppState;

#[actix_web::post("/api/voice/start")]
async fn voice_start(
    data: web::Data<AppState>,
    info: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let session_id = info
        .get("session_id")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let user_id = info
        .get("user_id")
        .and_then(|u| u.as_str())
        .unwrap_or("user");
    info!(
        "Voice session start request - session: {}, user: {}",
        session_id, user_id
    );

    match data
        .voice_adapter
        .start_voice_session(session_id, user_id)
        .await
    {
        Ok(token) => {
            info!(
                "Voice session started successfully for session {}",
                session_id
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({"token": token, "status": "started"})))
        }
        Err(e) => {
            error!(
                "Failed to start voice session for session {}: {}",
                session_id, e
            );
            Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()})))
        }
    }
}

#[actix_web::post("/api/voice/stop")]
async fn voice_stop(
    data: web::Data<AppState>,
    info: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let session_id = info
        .get("session_id")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    match data.voice_adapter.stop_voice_session(session_id).await {
        Ok(()) => {
            info!(
                "Voice session stopped successfully for session {}",
                session_id
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({"status": "stopped"})))
        }
        Err(e) => {
            error!(
                "Failed to stop voice session for session {}: {}",
                session_id, e
            );
            Ok(HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": e.to_string()})))
        }
    }
}
