//! Channel bindings and telephony logs (canonical, #1497).

diesel::table! {
    channel_bindings (bot_id) {
        bot_id -> Uuid,
        phone_default -> Nullable<Varchar>,
        whatsapp_number -> Nullable<Varchar>,
        telegram_username -> Nullable<Varchar>,
        domains -> Jsonb,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    call_logs (id) {
        id -> Uuid,
        bot_id -> Uuid,
        direction -> Varchar,
        from_number -> Nullable<Varchar>,
        to_number -> Nullable<Varchar>,
        status -> Varchar,
        duration_sec -> Nullable<Int4>,
        recording_ref -> Nullable<Text>,
        transcript -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(channel_bindings, call_logs);
