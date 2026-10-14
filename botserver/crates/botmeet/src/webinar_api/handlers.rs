use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;

use super::service::WebinarService;
use super::types::{
    AnswerQuestionRequest, RegisterRequest, SubmitQuestionRequest, Webinar, WebinarParticipant,
    WebinarRegistration, QAQuestion,
};
use super::error::WebinarError;

pub fn webinar_routes(_state: Arc<AppState>) -> axum::Router<Arc<AppState>> {
    axum::routing::Router::new()
        .route("/", post(create_webinar_handler))
        .route("/:id", get(get_webinar_handler))
        .route("/:id/start", post(start_webinar_handler))
        .route("/:id/end", post(end_webinar_handler))
        .route("/:id/register", post(register_handler))
        .route("/:id/join", post(join_handler))
        .route("/:id/hand/raise", post(raise_hand_handler))
        .route("/:id/hand/lower", post(lower_hand_handler))
        .route("/:id/hands", get(get_raised_hands_handler))
        .route("/:id/questions", get(get_questions_handler))
        .route("/:id/questions", post(submit_question_handler))
        .route("/:id/questions/:question_id/answer", post(answer_question_handler))
        .route("/:id/questions/:question_id/upvote", post(upvote_question_handler))
        // Recording and transcription routes
        .route("/:id/recording/start", post(start_recording_handler))
        .route("/:id/recording/stop", post(stop_recording_handler))
}

async fn start_recording_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> impl IntoResponse {
    let pool = state.conn.clone();
    let recording_id = Uuid::new_v4();
    let started_at = Utc::now();

    // Create recording record in database
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("DB error: {}", e))?;

        diesel::sql_query(
            "INSERT INTO meeting_recordings (id, room_id, status, started_at, created_at)
             VALUES ($1, $2, 'recording', $3, NOW())
             ON CONFLICT (room_id) WHERE status = 'recording' DO NOTHING"
        )
        .bind::<sql_types::Uuid, _>(recording_id)
        .bind::<sql_types::Uuid, _>(webinar_id)
        .bind::<sql_types::Timestamptz, _>(started_at)
        .execute(&mut conn)
        .map_err(|e| format!("Insert error: {}", e))?;

        Ok::<_, String>(recording_id)
    })
    .await;

    match result {
        Ok(Ok(id)) => Json(serde_json::json!({
            "status": "recording_started",
            "recording_id": id,
            "webinar_id": webinar_id,
            "started_at": started_at.to_rfc3339()
        })),
        Ok(Err(e)) => Json(serde_json::json!({
            "status": "error",
            "error": e
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botmeet::webinar_api::handlers", None)
        })),
    }
}

async fn stop_recording_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> impl IntoResponse {
    let pool = state.conn.clone();
    let stopped_at = Utc::now();

    // Update recording record to stopped status
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("DB error: {}", e))?;

        // Get the active recording and calculate duration
        let recording: Result<(Uuid, chrono::DateTime<chrono::Utc>), _> = diesel::sql_query(
            "SELECT id, started_at FROM meeting_recordings
             WHERE room_id = $1 AND status = 'recording'
             LIMIT 1"
        )
        .bind::<sql_types::Uuid, _>(webinar_id)
        .get_result::<RecordingRow>(&mut conn)
        .map(|r| (r.id, r.started_at));

        if let Ok((recording_id, started_at)) = recording {
            let duration_secs = (stopped_at - started_at).num_seconds();

            diesel::sql_query(
                "UPDATE meeting_recordings
                 SET status = 'stopped', stopped_at = $1, duration_seconds = $2, updated_at = NOW()
                 WHERE id = $3"
            )
            .bind::<sql_types::Timestamptz, _>(stopped_at)
            .bind::<sql_types::BigInt, _>(duration_secs)
            .bind::<sql_types::Uuid, _>(recording_id)
            .execute(&mut conn)
            .map_err(|e| format!("Update error: {}", e))?;

            Ok::<_, String>((recording_id, duration_secs))
        } else {
            Err("No active recording found".to_string())
        }
    })
    .await;

    match result {
        Ok(Ok((id, duration))) => Json(serde_json::json!({
            "status": "recording_stopped",
            "recording_id": id,
            "webinar_id": webinar_id,
            "stopped_at": stopped_at.to_rfc3339(),
            "duration_seconds": duration
        })),
        Ok(Err(e)) => Json(serde_json::json!({
            "status": "error",
            "error": e
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botmeet::webinar_api::handlers", None)
        })),
    }
}

#[derive(QueryableByName)]
struct RecordingRow {
    #[diesel(sql_type = sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = sql_types::Timestamptz)]
    started_at: chrono::DateTime<Utc>,
}

async fn create_webinar_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<super::types::CreateWebinarRequest>,
) -> Result<Json<Webinar>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let organization_id = Uuid::nil();
    let host_id = Uuid::nil();
    let webinar = service.create_webinar(organization_id, host_id, request).await?;
    Ok(Json(webinar))
}

async fn get_webinar_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> Result<Json<Webinar>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let webinar = service.get_webinar(webinar_id).await?;
    Ok(Json(webinar))
}

async fn start_webinar_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> Result<Json<Webinar>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let host_id = Uuid::nil();
    let webinar = service.start_webinar(webinar_id, host_id).await?;
    Ok(Json(webinar))
}

async fn end_webinar_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> Result<Json<Webinar>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let host_id = Uuid::nil();
    let webinar = service.end_webinar(webinar_id, host_id).await?;
    Ok(Json(webinar))
}

async fn register_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<WebinarRegistration>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let registration = service.register_attendee(webinar_id, request).await?;
    Ok(Json(registration))
}

async fn join_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> Result<Json<WebinarParticipant>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let participant_id = Uuid::nil();
    let participant = service.join_webinar(webinar_id, participant_id).await?;
    Ok(Json(participant))
}

async fn raise_hand_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let participant_id = Uuid::nil();
    service.raise_hand(webinar_id, participant_id).await?;
    Ok(axum::http::StatusCode::OK)
}

async fn lower_hand_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let participant_id = Uuid::nil();
    service.lower_hand(webinar_id, participant_id).await?;
    Ok(axum::http::StatusCode::OK)
}

async fn get_raised_hands_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> Result<Json<Vec<WebinarParticipant>>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let hands = service.get_raised_hands(webinar_id).await?;
    Ok(Json(hands))
}

async fn get_questions_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
) -> Result<Json<Vec<QAQuestion>>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let questions = service.get_questions(webinar_id, false).await?;
    Ok(Json(questions))
}

async fn submit_question_handler(
    State(state): State<Arc<AppState>>,
    Path(webinar_id): Path<Uuid>,
    Json(request): Json<SubmitQuestionRequest>,
) -> Result<Json<QAQuestion>, WebinarError> {
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let asker_id: Option<Uuid> = None;
    let question = service.submit_question(webinar_id, asker_id, "Anonymous".to_string(), request).await?;
    Ok(Json(question))
}

async fn answer_question_handler(
    State(state): State<Arc<AppState>>,
    Path((webinar_id, question_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AnswerQuestionRequest>,
) -> Result<Json<QAQuestion>, WebinarError> {
    log::debug!("Answering question {question_id} in webinar {webinar_id}");
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let answerer_id = Uuid::nil();
    let question = service.answer_question(question_id, answerer_id, request).await?;
    Ok(Json(question))
}

async fn upvote_question_handler(
    State(state): State<Arc<AppState>>,
    Path((webinar_id, question_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<QAQuestion>, WebinarError> {
    log::debug!("Upvoting question {question_id} in webinar {webinar_id}");
    let service = WebinarService::new(Arc::new(state.conn.clone()));
    let voter_id = Uuid::nil();
    let question = service.upvote_question(question_id, voter_id).await?;
    Ok(Json(question))
}
