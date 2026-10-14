use super::*;

pub async fn handle_mute_all(
    Form(payload): Form<MuteAllRequest>,
) -> impl IntoResponse {
    let room = payload.room_id.unwrap_or_else(|| "default".to_string());
    info!("Muting all participants in room {room}");
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "muted", "room_id": room })),
    )
}
pub async fn handle_voice_toggle() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "toggled" }))
}
pub async fn start_transcription(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    let transcription_service = Arc::new(DefaultTranscriptionService);
    let meeting_service = MeetingService::new(state.clone(), transcription_service);

    match meeting_service.start_transcription(&room_id).await {
        Ok(()) => {
            info!("Started transcription for room {room_id}");
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "transcription_started"})),
            )
        }
        Err(e) => {
            error!("Failed to start transcription for room {room_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": botsecurity::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botmeet::lib_part4", None)})),
            )
        }
    }
}
pub async fn get_meeting_token(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<GetTokenRequest>,
) -> impl IntoResponse {
    let token = format!(
        "meet_token_{}_{}_{}",
        payload.room_id,
        payload.user_id,
        uuid::Uuid::new_v4()
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "token": token,
            "room_id": payload.room_id,
            "user_id": payload.user_id
        })),
    )
}
pub async fn send_meeting_invites(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<SendInvitesRequest>,
) -> impl IntoResponse {
    info!("Sending meeting invites for room {}", payload.room_id);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "invites_sent",
            "recipients": payload.emails
        })),
    )
}
pub async fn meeting_websocket(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_meeting_socket(socket, state))
}
pub(super) async fn handle_meeting_socket(socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    info!("Meeting WebSocket connection established");
    let (mut sender, mut receiver) = socket.split();

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                info!("Meeting message received: {text}");
                if sender
                    .send(axum::extract::ws::Message::Text(format!("Echo: {text}")))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok(axum::extract::ws::Message::Close(_)) => break,
            Err(e) => {
                error!("WebSocket error: {e}");
                break;
            }
            _ => {}
        }
    }

    drop(state);
}
pub async fn signaling_websocket(
    ws: axum::extract::ws::WebSocketUpgrade,
    Path(room_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rooms = state
        .extensions
        .get::<SignalingRooms>()
        .await
        .map(|arc| (*arc).clone())
        .unwrap_or_else(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())));
    ws.on_upgrade(move |socket| {
        signaling::handle_signaling_socket(socket, room_id, state, rooms)
    })
}
pub async fn turn_credentials() -> impl IntoResponse {
    let config = turn::TurnConfig::from_env();
    let credentials = turn::get_turn_credentials(&config);
    Json(credentials)
}
pub async fn schedule_meeting_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScheduleMeetingRequest>,
) -> impl IntoResponse {
    match room_persistence::schedule_meeting(&state, request).await {
        Ok(room) => {
            info!("Scheduled meeting: {} ({})", room.title, room.id);
            (StatusCode::OK, Json(serde_json::json!(room)))
        }
        Err(e) => {
            error!("Failed to schedule meeting: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botmeet::lib_part4", None)})),
            )
        }
    }
}
pub async fn dashboard_stats(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match room_persistence::get_dashboard_stats(&state).await {
        Ok(stats) => (StatusCode::OK, Json(serde_json::json!(stats))),
        Err(e) => {
            error!("Failed to get dashboard stats: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botmeet::lib_part4", None)})),
            )
        }
    }
}
pub async fn list_meetings_htmx(
    State(state): State<Arc<AppState>>,
) -> Html<String> {
    let query = room_persistence::DashboardQuery {
        status: None,
        limit: Some(20),
        offset: Some(0),
    };

    match room_persistence::list_meetings(&state, query).await {
        Ok(meetings) => {
            if meetings.is_empty() {
                return Html(
                    r#"<div class="empty-state"><p>No meetings found</p></div>"#
                        .to_string(),
                );
            }
            let mut html = String::new();
            for m in &meetings {
                let status_class = match m.status.as_str() {
                    "live" => "status-live",
                    "scheduled" => "status-scheduled",
                    _ => "status-ended",
                };
                html.push_str(&format!(
                    r#"<div class="meeting-card {status_class}" data-id="{id}">
                        <h4>{title}</h4>
                        <p>Status: {status} | Participants: {participants}</p>
                    </div>"#,
                    id = m.id,
                    title = m.title,
                    status = m.status,
                    participants = m.participant_count,
                ));
            }
            Html(html)
        }
        Err(e) => Html(format!("<div class='error'>Error: {e}</div>")),
    }
}
pub async fn all_participants(State(_state): State<Arc<AppState>>) -> Html<String> {
    Html(r##"<div class="empty-state">
        <p>No participants</p>
    </div>"##.to_string())
}
pub async fn recent_meetings(State(_state): State<Arc<AppState>>) -> Html<String> {
    Html(r##"<div class="empty-state">
        <div class="empty-icon">📋</div>
        <p>No recent meetings</p>
    </div>"##.to_string())
}
pub async fn scheduled_meetings(State(_state): State<Arc<AppState>>) -> Html<String> {
    Html(r##"<div class="empty-state">
        <div class="empty-icon">📅</div>
        <p>No scheduled meetings</p>
    </div>"##.to_string())
}
