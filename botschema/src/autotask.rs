//! AutoTask / agent-safety tables (canonical, #1497).

diesel::table! {
    safety_constraints (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        name -> Text,
        constraint_type -> Varchar,
        description -> Nullable<Text>,
        expression -> Nullable<Text>,
        threshold -> Nullable<Text>,
        severity -> Text,
        enabled -> Bool,
        applies_to -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    audit_log (id) {
        id -> Uuid,
        timestamp -> Timestamptz,
        event_type -> Varchar,
        actor_type -> Varchar,
        actor_id -> Uuid,
        action -> Varchar,
        target_type -> Nullable<Varchar>,
        target_id -> Nullable<Uuid>,
        outcome_success -> Bool,
        details -> Nullable<Text>,
        session_id -> Nullable<Uuid>,
        bot_id -> Uuid,
        task_id -> Nullable<Uuid>,
        step_id -> Nullable<Uuid>,
        risk_level -> Varchar,
    }
}

diesel::table! {
    intent_classifications (id) {
        id -> Uuid,
        bot_id -> Uuid,
        session_id -> Nullable<Uuid>,
        original_text -> Text,
        intent_type -> Varchar,
        confidence -> Float8,
        entities -> Nullable<Jsonb>,
        suggested_name -> Nullable<Varchar>,
        was_correct -> Nullable<Bool>,
        corrected_type -> Nullable<Varchar>,
        feedback -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
        branch_id -> Uuid,
    }
}

diesel::table! {
    designer_changes (id) {
        id -> Uuid,
        bot_id -> Uuid,
        session_id -> Nullable<Uuid>,
        change_type -> Varchar,
        description -> Text,
        file_path -> Varchar,
        original_content -> Text,
        new_content -> Text,
        created_at -> Nullable<Timestamptz>,
        branch_id -> Uuid,
    }
}

diesel::table! {
    designer_pending_changes (id) {
        id -> Uuid,
        bot_id -> Uuid,
        session_id -> Nullable<Uuid>,
        analysis_json -> Text,
        instruction -> Text,
        expires_at -> Timestamptz,
        created_at -> Nullable<Timestamptz>,
    }
}
