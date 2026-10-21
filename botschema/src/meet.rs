use crate::core::{branches};

diesel::table! {
    meeting_rooms (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 50]
    room_code -> Varchar,
    #[max_length = 255]
    name -> Varchar,
    description -> Nullable<Text>,
    created_by -> Uuid,
    max_participants -> Int4,
    is_recording -> Bool,
    is_transcribing -> Bool,
    #[max_length = 20]
    status -> Varchar,
    settings -> Jsonb,
    started_at -> Nullable<Timestamptz>,
    ended_at -> Nullable<Timestamptz>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    meeting_participants (id) {
        id -> Uuid,
        room_id -> Uuid,
        user_id -> Nullable<Uuid>,
        participant_name -> Varchar,
        email -> Nullable<Varchar>,
        role -> Varchar,
        is_bot -> Bool,
        is_active -> Bool,
        has_video -> Bool,
        has_audio -> Bool,
        joined_at -> Timestamptz,
        left_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    meeting_recordings (id) {

    id -> Uuid,
    room_id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 20]
    recording_type -> Varchar,
    file_url -> Nullable<Text>,
    file_size -> Nullable<Int8>,
    duration_seconds -> Nullable<Int4>,
    #[max_length = 20]
    status -> Varchar,
    started_at -> Timestamptz,
    stopped_at -> Nullable<Timestamptz>,
    processed_at -> Nullable<Timestamptz>,
    metadata -> Jsonb,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    meeting_transcriptions (id) {

    id -> Uuid,
    room_id -> Uuid,
    recording_id -> Nullable<Uuid>,
    org_id -> Uuid,
    bot_id -> Uuid,
    participant_id -> Nullable<Uuid>,
    #[max_length = 255]
    speaker_name -> Nullable<Varchar>,
    content -> Text,
    start_time -> Numeric,
    end_time -> Numeric,
    confidence -> Nullable<Numeric>,
    #[max_length = 10]
    language -> Nullable<Varchar>,
    is_final -> Bool,
    metadata -> Jsonb,
    created_at -> Timestamptz,
    branch_id -> Nullable<Uuid>,
    }
}


diesel::table! {
    meeting_whiteboards (id) {

    id -> Uuid,
    room_id -> Nullable<Uuid>,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    #[max_length = 20]
    background_color -> Nullable<Varchar>,
    grid_enabled -> Bool,
    grid_size -> Nullable<Int4>,
    elements -> Jsonb,
    version -> Int4,
    created_by -> Uuid,
    last_modified_by -> Nullable<Uuid>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    whiteboard_elements (id) {
        id -> Uuid,
        whiteboard_id -> Uuid,
        element_type -> Varchar,
        position_x -> Numeric,
        position_y -> Numeric,
        width -> Nullable<Numeric>,
        height -> Nullable<Numeric>,
        rotation -> Nullable<Numeric>,
        z_index -> Int4,
        properties -> Jsonb,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    whiteboard_exports (id) {

    id -> Uuid,
    whiteboard_id -> Uuid,
    org_id -> Uuid,
    #[max_length = 20]
    export_format -> Varchar,
    file_url -> Nullable<Text>,
    file_size -> Nullable<Int8>,
    #[max_length = 20]
    status -> Varchar,
    error_message -> Nullable<Text>,
    requested_by -> Uuid,
    created_at -> Timestamptz,
    completed_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    meeting_chat_messages (id) {
        id -> Uuid,
        room_id -> Uuid,
        participant_id -> Nullable<Uuid>,
        sender_name -> Varchar,
        message_type -> Varchar,
        content -> Text,
        reply_to_id -> Nullable<Uuid>,
        is_system_message -> Bool,
        metadata -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    scheduled_meetings (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    room_id -> Nullable<Uuid>,
    #[max_length = 255]
    title -> Varchar,
    description -> Nullable<Text>,
    organizer_id -> Uuid,
    scheduled_start -> Timestamptz,
    scheduled_end -> Timestamptz,
    #[max_length = 50]
    timezone -> Varchar,
    recurrence_rule -> Nullable<Text>,
    attendees -> Jsonb,
    settings -> Jsonb,
    #[max_length = 20]
    status -> Varchar,
    reminder_sent -> Bool,
    calendar_event_id -> Nullable<Uuid>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::joinable!(meeting_rooms -> branches (branch_id));
diesel::joinable!(meeting_participants -> meeting_rooms (room_id));
diesel::joinable!(meeting_recordings -> meeting_rooms (room_id));
diesel::joinable!(meeting_recordings -> branches (branch_id));
diesel::joinable!(meeting_transcriptions -> meeting_rooms (room_id));
diesel::joinable!(meeting_transcriptions -> meeting_recordings (recording_id));
diesel::joinable!(meeting_transcriptions -> meeting_participants (participant_id));
diesel::joinable!(meeting_whiteboards -> meeting_rooms (room_id));
diesel::joinable!(meeting_whiteboards -> branches (branch_id));
diesel::joinable!(whiteboard_elements -> meeting_whiteboards (whiteboard_id));
diesel::joinable!(whiteboard_exports -> meeting_whiteboards (whiteboard_id));
diesel::joinable!(whiteboard_exports -> branches (branch_id));
diesel::joinable!(meeting_chat_messages -> meeting_rooms (room_id));
diesel::joinable!(meeting_chat_messages -> meeting_participants (participant_id));
diesel::joinable!(scheduled_meetings -> branches (branch_id));
diesel::joinable!(scheduled_meetings -> meeting_rooms (room_id));
