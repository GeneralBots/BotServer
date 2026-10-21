//! Video editor tables (canonical, #1497).

diesel::table! {
    video_projects (id) {
        id -> Uuid,
        organization_id -> Nullable<Uuid>,
        created_by -> Nullable<Uuid>,
        name -> Text,
        description -> Nullable<Text>,
        resolution_width -> Int4,
        resolution_height -> Int4,
        fps -> Int4,
        total_duration_ms -> Int8,
        timeline_json -> Jsonb,
        layers_json -> Jsonb,
        audio_tracks_json -> Jsonb,
        playhead_ms -> Int8,
        selection_json -> Jsonb,
        zoom_level -> Float4,
        thumbnail_url -> Nullable<Text>,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    video_clips (id) {
        id -> Uuid,
        project_id -> Uuid,
        name -> Text,
        source_url -> Text,
        start_ms -> Int8,
        duration_ms -> Int8,
        trim_in_ms -> Int8,
        trim_out_ms -> Int8,
        volume -> Float4,
        clip_order -> Int4,
        transition_in -> Nullable<Text>,
        transition_out -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    video_layers (id) {

    id -> Uuid,
    project_id -> Uuid,
    name -> Text,
    layer_type -> Text,
    track_index -> Int4,
    start_ms -> Int8,
    end_ms -> Int8,
    x -> Float4,
    y -> Float4,
    width -> Float4,
    height -> Float4,
    rotation -> Float4,
    opacity -> Float4,
    properties_json -> Jsonb,
    animation_in -> Nullable<Text>,
    animation_out -> Nullable<Text>,
    locked -> Bool,
    created_at -> Timestamptz,
    keyframes_json -> Nullable<Jsonb>,
    }
}



diesel::table! {
    video_audio_tracks (id) {

    id -> Uuid,
    project_id -> Uuid,
    name -> Text,
    source_url -> Text,
    track_type -> Text,
    start_ms -> Int8,
    duration_ms -> Int8,
    volume -> Float4,
    fade_in_ms -> Int8,
    fade_out_ms -> Int8,
    created_at -> Timestamptz,
    waveform_json -> Nullable<Jsonb>,
    beat_markers_json -> Nullable<Jsonb>,
    }
}



diesel::table! {
    video_exports (id) {

    id -> Uuid,
    project_id -> Uuid,
    format -> Text,
    quality -> Text,
    status -> Text,
    progress -> Int4,
    output_url -> Nullable<Text>,
    error_message -> Nullable<Text>,
    created_at -> Timestamptz,
    completed_at -> Nullable<Timestamptz>,
    gbdrive_path -> Nullable<Text>,
    }
}



diesel::table! {
    video_command_history (id) {
        id -> Uuid,
        project_id -> Uuid,
        user_id -> Nullable<Uuid>,
        command_type -> Text,
        command_json -> Jsonb,
        executed_at -> Timestamptz,
    }
}

diesel::table! {
    video_analytics (id) {
        id -> Uuid,
        project_id -> Uuid,
        export_id -> Nullable<Uuid>,
        views -> Int8,
        unique_viewers -> Int8,
        total_watch_time_ms -> Int8,
        avg_watch_percent -> Float4,
        completions -> Int8,
        shares -> Int8,
        likes -> Int8,
        engagement_score -> Float4,
        viewer_retention_json -> Nullable<Jsonb>,
        geography_json -> Nullable<Jsonb>,
        device_json -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    video_keyframes (id) {
        id -> Uuid,
        layer_id -> Uuid,
        property_name -> Text,
        time_ms -> Int8,
        value_json -> Jsonb,
        easing -> Text,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(video_clips -> video_projects (project_id));
diesel::joinable!(video_layers -> video_projects (project_id));
diesel::joinable!(video_audio_tracks -> video_projects (project_id));
diesel::joinable!(video_exports -> video_projects (project_id));
diesel::joinable!(video_command_history -> video_projects (project_id));
diesel::joinable!(video_analytics -> video_projects (project_id));
diesel::joinable!(video_keyframes -> video_layers (layer_id));

diesel::allow_tables_to_appear_in_same_query!(
    video_projects,
    video_clips,
    video_layers,
    video_audio_tracks,
    video_exports,
    video_command_history,
    video_analytics,
    video_keyframes,
);
