use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use log::error;
use serde::Deserialize;
use std::sync::Arc;

use botcore::shared::state::AppState;
use crate::room_persistence;
use crate::{service::DefaultTranscriptionService, service::MeetingService};

#[derive(Debug, Deserialize)]
pub struct DashboardListQuery {
    pub view: Option<String>,
}

pub async fn dashboard_stats_htmx(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match room_persistence::get_dashboard_stats(&state).await {
        Ok(stats) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "live_count": stats.live_meetings,
                "today_count": stats.scheduled_meetings + stats.live_meetings,
                "week_count": stats.total_meetings,
                "total_hours": stats.total_recording_hours,
                "total_participants": stats.total_participants,
            })),
        ),
        Err(e) => {
            error!("Failed to get dashboard stats: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botmeet::dashboard", None)})),
            )
        }
    }
}

pub async fn dashboard_list_htmx(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DashboardListQuery>,
) -> Html<String> {
    let view = query.view.as_deref().unwrap_or("upcoming");
    let status_filter = match view {
        "live" => Some("live".to_string()),
        "past" => Some("ended".to_string()),
        "recordings" => None,
        _ => Some("scheduled".to_string()),
    };

    let db_query = room_persistence::DashboardQuery {
        status: status_filter,
        limit: Some(20),
        offset: Some(0),
    };

    let meetings = match room_persistence::list_meetings(&state, db_query).await {
        Ok(m) => m,
        Err(e) => return Html(format!("<div class='error'>Error: {e}</div>")),
    };

    if meetings.is_empty() {
        return Html(render_empty_state(view));
    }

    let mut html = String::from("<div class='meeting-grid'>");
    for m in &meetings {
        html.push_str(&render_meeting_card(m));
    }
    html.push_str("</div>");
    Html(html)
}

pub async fn create_room_htmx(
    State(state): State<Arc<AppState>>,
    axum::extract::Form(payload): axum::extract::Form<CreateRoomForm>,
) -> impl IntoResponse {
    let name = payload.title
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| format!("Meeting {}", chrono::Utc::now().format("%H:%M")));
    let created_by = payload.created_by.unwrap_or_else(|| "user".into());

    let transcription_service = Arc::new(DefaultTranscriptionService);
    let meeting_service = MeetingService::new(state.clone(), transcription_service);

    match meeting_service.create_room(name, created_by, None).await {
        Ok(room) => {
            let card = render_meeting_card(&room_persistence::MeetingListItem {
                id: uuid::Uuid::parse_str(&room.id).unwrap_or_default(),
                title: room.name,
                description: None,
                status: "live".to_string(),
                scheduled_at: None,
                duration_minutes: 60,
                created_at: room.created_at,
                ended_at: None,
                is_recording: false,
                participant_count: 0,
            });
            (StatusCode::OK, Html(card))
        }
        Err(e) => {
            error!("Failed to create room: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("<div class='error'>Failed to create: {e}</div>")),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRoomForm {
    pub title: Option<String>,
    pub created_by: Option<String>,
}

fn render_empty_state(view: &str) -> String {
    let (icon, message) = match view {
        "live" => ("🔴", "No live meetings right now"),
        "past" => ("📋", "No past meetings found"),
        "recordings" => ("🎥", "No recordings available"),
        _ => ("📅", "No upcoming meetings scheduled"),
    };
    format!(
        r#"<div class='empty-state'>
            <div class='empty-icon'>{icon}</div>
            <p>{message}</p>
            <p class='empty-hint'>Create a new meeting to get started</p>
        </div>"#
    )
}

fn render_meeting_card(m: &room_persistence::MeetingListItem) -> String {
    let status_class = match m.status.as_str() {
        "live" => "status-live",
        "scheduled" => "status-scheduled",
        _ => "status-ended",
    };
    let status_label = match m.status.as_str() {
        "live" => "Live Now",
        "scheduled" => "Scheduled",
        _ => "Ended",
    };
    let time_str = m.scheduled_at
        .map(|dt| dt.format("%b %d, %H:%M").to_string())
        .unwrap_or_else(|| m.created_at.format("%b %d, %H:%M").to_string());
    let join_btn = if m.status == "live" {
        format!(
            r#"<button class='btn btn-success btn-join' hx-post='/api/meet/rooms/{}/join' hx-swap='none'>Join Now</button>"#,
            m.id
        )
    } else if m.status == "scheduled" {
        format!(
            r#"<button class='btn btn-primary' hx-post='/api/meet/rooms/{}/join' hx-swap='none'>Start</button>"#,
            m.id
        )
    } else {
        String::new()
    };

    format!(
        r#"<div class='meeting-card {status_class}' data-id='{id}'>
            <div class='meeting-header'>
                <span class='meeting-status {status_class}'>{status}</span>
                <span class='meeting-duration'>{dur} min</span>
            </div>
            <h4 class='meeting-title'>{title}</h4>
            <div class='meeting-meta'>
                <span class='meeting-time'>{time}</span>
                <span class='meeting-participants'>{pc} participant(s)</span>
            </div>
            <div class='meeting-actions'>
                {join_btn}
                <button class='btn btn-outline' onclick='navigator.clipboard.writeText(location.origin + "/suite/meet/room/{id}")'>Copy Link</button>
            </div>
        </div>"#,
        id = m.id,
        status_class = status_class,
        status = status_label,
        dur = m.duration_minutes,
        title = m.title,
        time = time_str,
        pc = m.participant_count,
        join_btn = join_btn,
    )
}
