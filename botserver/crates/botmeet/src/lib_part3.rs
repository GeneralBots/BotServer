use super::*;

pub async fn handle_meet_join_htmx(
    Form(payload): Form<MeetJoinHtmxRequest>,
) -> impl IntoResponse {
    let name = payload
        .user_name
        .unwrap_or_else(|| "Guest".to_string());
    let room = payload
        .room_id
        .or(payload.meeting_code)
        .unwrap_or_else(|| "default".to_string());
    let escaped_name = name.replace('<', "&lt;").replace('>', "&gt;");
    let escaped_room = room.replace('<', "&lt;").replace('>', "&gt;");

    let html = format!(
        r##"<div class="meeting-room" id="meeting-room">
        <header class="room-header">
            <div class="room-info">
                <h2 id="room-title">Meeting Room</h2>
                <span class="room-id">Room: {room}</span>
                <span class="room-participant">Joined as {name}</span>
            </div>
            <div class="room-actions">
                <button class="btn-secondary" hx-post="/api/meet/mute-all" hx-swap="none">Mute All</button>
                <button class="btn-secondary" onclick="leaveMeeting()">Leave</button>
            </div>
        </header>
        <div class="meeting-grid">
            <div class="participant-tile local">
                <video autoplay muted playsinline></video>
                <span class="participant-name">{name}</span>
            </div>
        </div>
        </div>"##,
        room = escaped_room,
        name = escaped_name,
    );

    Html(html)
}
