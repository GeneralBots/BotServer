use super::*;

pub fn configure() -> Router<Arc<AppState>> {
    Router::new()
        .route(ApiUrls::VOICE_START, post(voice_start))
        .route(ApiUrls::VOICE_STOP, post(voice_stop))
        .route(ApiUrls::MEET_CREATE, post(create_meeting))
        .route(ApiUrls::MEET_ROOMS, get(list_rooms))
        .route(ApiUrls::MEET_ROOM_BY_ID, get(get_room))
        .route(ApiUrls::MEET_JOIN, post(join_room))
        .route(ApiUrls::MEET_TRANSCRIPTION, post(start_transcription))
        .route(ApiUrls::MEET_RECORDING_START, post(recording_routes::start_recording))
        .route(ApiUrls::MEET_RECORDING_STOP, post(recording_routes::stop_recording))
        .route(ApiUrls::MEET_RECORDING_PAUSE, post(recording_routes::pause_recording))
        .route(ApiUrls::MEET_RECORDING_RESUME, post(recording_routes::resume_recording))
        .route(ApiUrls::MEET_RECORDINGS_LIST, get(recording_routes::list_recordings))
        .route(ApiUrls::MEET_RECORDING_GET, get(recording_routes::get_recording))
        .route(ApiUrls::MEET_RECORDING_FILE, get(recording_routes::get_recording_file))
        .route(ApiUrls::MEET_RECORDING_DELETE, delete(recording_routes::delete_recording))
        .route(ApiUrls::MEET_TOKEN, post(get_meeting_token))
        .route(ApiUrls::MEET_INVITE, post(send_meeting_invites))
        .route(ApiUrls::WS_MEET, get(meeting_websocket))
        .route(
            ApiUrls::WS_MEET_SIGNALING,
            get(signaling_websocket),
        )
        .route(
            ApiUrls::MEET_TURN_CREDENTIALS,
            get(turn_credentials),
        )
        .route(ApiUrls::MEET_SCHEDULE, post(schedule_meeting_handler))
        .route(ApiUrls::MEET_DASHBOARD_STATS, get(dashboard_stats))
        .route(ApiUrls::MEET_LEAVE, post(leave_room))
        .route(ApiUrls::MEET_ROOMS_HTMX, get(list_rooms))
        .route(ApiUrls::MEET_PARTICIPANTS_HTMX, get(all_participants))
        .route(ApiUrls::MEET_RECENT_HTMX, get(recent_meetings))
        .route(ApiUrls::MEET_SCHEDULED_HTMX, get(scheduled_meetings))
        .route(ApiUrls::MEET_DASHBOARD_LIST_HTMX, get(list_meetings_htmx))
        .route(
            "/conversations/create",
            post(conversations::create_conversation),
        )
        .route(
            "/conversations/:id/join",
            post(conversations::join_conversation),
        )
        .route(
            "/conversations/:id/leave",
            post(conversations::leave_conversation),
        )
        .route(
            "/conversations/:id/members",
            get(conversations::get_conversation_members),
        )
        .route(
            "/conversations/:id/messages",
            get(conversations::get_conversation_messages),
        )
        .route(
            "/conversations/:id/messages/send",
            post(conversations::send_message),
        )
        .route(
            "/conversations/:id/messages/:message_id/edit",
            post(conversations::edit_message),
        )
        .route(
            "/conversations/:id/messages/:message_id/delete",
            post(conversations::delete_message),
        )
        .route(
            "/conversations/:id/messages/:message_id/react",
            post(conversations::react_to_message),
        )
        .route(
            "/conversations/:id/messages/:message_id/pin",
            post(conversations::pin_message),
        )
        .route(
            "/conversations/:id/messages/search",
            get(conversations::search_messages),
        )
        .route(
            "/conversations/:id/calls/start",
            post(conversations::start_call),
        )
        .route(
            "/conversations/:id/calls/join",
            post(conversations::join_call),
        )
        .route(
            "/conversations/:id/calls/leave",
            post(conversations::leave_call),
        )
        .route(
            "/conversations/:id/calls/mute",
            post(conversations::mute_call),
        )
        .route(
            "/conversations/:id/calls/unmute",
            post(conversations::unmute_call),
        )
        .route(
            "/conversations/:id/screen/share",
            post(conversations::start_screen_share),
        )
        .route(
            "/conversations/:id/screen/stop",
            post(conversations::stop_screen_share),
        )
        .route(
            "/conversations/:id/recording/start",
            post(conversations::start_recording),
        )
        .route(
            "/conversations/:id/recording/stop",
            post(conversations::stop_recording),
        )
        .route(
            "/conversations/:id/whiteboard/create",
            post(conversations::create_whiteboard),
        )
        .route(
            "/conversations/:id/whiteboard/collaborate",
            post(conversations::collaborate_whiteboard),
        )
        .route(
            ApiUrls::MEET_HTMX_DASHBOARD_STATS,
            get(dashboard::dashboard_stats_htmx),
        )
        .route(
            ApiUrls::MEET_HTMX_DASHBOARD_LIST,
            get(dashboard::dashboard_list_htmx),
        )
        .route(
            ApiUrls::MEET_HTMX_ROOM_CREATE,
            post(dashboard::create_room_htmx),
        )
        .merge(minutes::handlers::minutes_routes())
        .route("/api/meet/join", post(handle_meet_join_htmx))
        .route("/api/meet/mute-all", post(handle_mute_all))
        .route("/api/voice/toggle", post(handle_voice_toggle))
        .merge(trunks::trunks_routes())
}
