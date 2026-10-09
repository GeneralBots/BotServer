use super::*;

pub async fn voice_start(
    State(data): State<Arc<AppState>>,
    Json(info): Json<Value>,
) -> impl IntoResponse {
    let session_id = info
        .get("session_id")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let user_id = info
        .get("user_id")
        .and_then(|u| u.as_str())
        .unwrap_or("user");

    info!("Voice session start request - session: {session_id}, user: {user_id}");

    match data
        .voice_adapter
        .start_voice_session(session_id, user_id)
        .await
    {
        Ok(token) => {
            info!("Voice session started successfully for session {session_id}");
            (
                StatusCode::OK,
                Json(serde_json::json!({"token": token, "status": "started"})),
            )
        }
        Err(e) => {
            error!("Failed to start voice session for session {session_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    }
}
pub async fn voice_stop(
    State(data): State<Arc<AppState>>,
    Json(info): Json<Value>,
) -> impl IntoResponse {
    let session_id = info
        .get("session_id")
        .and_then(|s| s.as_str())
        .unwrap_or("");

    match data.voice_adapter.stop_voice_session(session_id).await {
        Ok(()) => {
            info!("Voice session stopped successfully for session {session_id}");
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "stopped"})),
            )
        }
        Err(e) => {
            error!("Failed to stop voice session for session {session_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    }
}
pub async fn create_meeting(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateMeetingRequest>,
) -> impl IntoResponse {
    let transcription_service = Arc::new(DefaultTranscriptionService);
    let meeting_service = MeetingService::new(state.clone(), transcription_service);

    match meeting_service
        .create_room(payload.name, payload.created_by, payload.settings)
        .await
    {
        Ok(room) => {
            info!("Created meeting room: {}", room.id);
            (StatusCode::OK, Json(serde_json::json!(room)))
        }
        Err(e) => {
            error!("Failed to create meeting room: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    }
}
pub async fn list_rooms(State(state): State<Arc<AppState>>) -> Html<String> {
    let transcription_service = Arc::new(DefaultTranscriptionService);
    let meeting_service = MeetingService::new(state.clone(), transcription_service);

    let rooms = meeting_service.rooms.read().await;

    if rooms.is_empty() {
        return Html(r##"<div class="empty-state">
            <div class="empty-icon">📹</div>
            <p>No active rooms</p>
            <p class="empty-hint">Create a new meeting to get started</p>
        </div>"##.to_string());
    }

    let mut html = String::new();
    for room in rooms.values() {
        let participant_count = room.participants.len();
        html.push_str(&format!(
            r##"<div class="room-card" data-room-id="{id}">
                <div class="room-icon">📹</div>
                <div class="room-info">
                    <h3 class="room-name">{name}</h3>
                    <span class="room-participants">{count} participant(s)</span>
                </div>
                <button class="btn-join" hx-post="/api/meet/rooms/{id}/join" hx-target="#meeting-room" hx-swap="outerHTML">Join</button>
            </div>"##,
            id = room.id,
            name = room.name,
            count = participant_count,
        ));
    }

    Html(html)
}
pub async fn get_room(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    let transcription_service = Arc::new(DefaultTranscriptionService);
    let meeting_service = MeetingService::new(state.clone(), transcription_service);

    let rooms = meeting_service.rooms.read().await;
    match rooms.get(&room_id) {
        Some(room) => (StatusCode::OK, Json(serde_json::json!(room))),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Room not found"})),
        ),
    }
}
pub async fn join_room(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(payload): Json<JoinRoomRequest>,
) -> impl IntoResponse {
    let transcription_service = Arc::new(DefaultTranscriptionService);
    let meeting_service = MeetingService::new(state.clone(), transcription_service);

    match meeting_service
        .join_room(&room_id, payload.participant_name, payload.participant_id)
        .await
    {
        Ok(participant) => {
            info!("Participant {} joined room {room_id}", participant.id);
            (StatusCode::OK, Json(serde_json::json!(participant)))
        }
        Err(e) => {
            error!("Failed to join room {room_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    }
}
pub async fn leave_room(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(payload): Json<LeaveRoomRequest>,
) -> impl IntoResponse {
    let rooms = state
        .extensions
        .get::<SignalingRooms>()
        .await
        .map(|arc| (*arc).clone())
        .unwrap_or_else(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())));

    let mut rooms_guard = rooms.write().await;
    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            if room.participants.remove(&payload.participant_id).is_some() {
                info!(
                    "Participant {} left room {room_id}",
                    payload.participant_id
                );
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "left",
                        "room_id": room_id,
                        "participant_id": payload.participant_id,
                        "remaining_participants": room.participants.len(),
                    })),
                )
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Participant not found in room"})),
                )
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Room not found"})),
        ),
    }
}
