pub mod safety_constraints {
    // Canonical shape created by migration `6.7.8-safety-constraints`. Only the
    // columns `SafetyLayer::load_constraints` reads are declared; `pattern`,
    // `action` and `is_active` were present in the previous declaration but
    // never queried and duplicated `expression`/`threshold`/`enabled`.
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
}

pub mod audit_log {
    diesel::table! {
        audit_log (id) {
            id -> Text,
            timestamp -> Text,
            event_type -> Text,
            actor_type -> Text,
            actor_id -> Text,
            action -> Text,
            target_type -> Text,
            target_id -> Text,
            outcome_success -> Bool,
            details -> Text,
            session_id -> Text,
            bot_id -> Text,
            task_id -> Nullable<Text>,
            step_id -> Nullable<Text>,
            risk_level -> Text,
        }
    }
}

pub mod intent_classifications {
    diesel::table! {
        intent_classifications (id) {
            id -> Uuid,
            branch_id -> Uuid,
            bot_id -> Uuid,
            intent_id -> Uuid,
            input -> Text,
            confidence -> Nullable<Float4>,
            classified_at -> Timestamptz,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
            session_id -> Uuid,
            original_text -> Text,
            intent_type -> Text,
            entities -> Text,
        }
    }
}

pub mod tasks {
    diesel::table! {
        tasks (id) {
            id -> Uuid,
            branch_id -> Uuid,
            title -> Varchar,
            description -> Nullable<Text>,
            status -> Nullable<Varchar>,
            priority -> Nullable<Int4>,
            assignee_id -> Nullable<Uuid>,
            due_date -> Nullable<Timestamptz>,
            completed_at -> Nullable<Timestamptz>,
            parent_id -> Nullable<Uuid>,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
            reporter_id -> Nullable<Uuid>,
            project_id -> Nullable<Uuid>,
            tags -> Text,
            dependencies -> Text,
            estimated_hours -> Nullable<Float8>,
            actual_hours -> Nullable<Float8>,
            progress -> Int4,
        }
    }
}

pub mod designer_changes {
    diesel::table! {
        designer_changes (id) {
            id -> Uuid,
            branch_id -> Uuid,
            bot_id -> Uuid,
            change_type -> Varchar,
            target_type -> Varchar,
            target_id -> Nullable<Uuid>,
            payload -> Jsonb,
            applied -> Nullable<Bool>,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
            description -> Text,
            file_path -> Text,
            original_content -> Text,
            new_content -> Text,
        }
    }
}

pub mod designer_pending_changes {
    diesel::table! {
        designer_pending_changes (id) {
            id -> Text,
            bot_id -> Uuid,
            analysis_json -> Text,
            expires_at -> Timestamptz,
        }
    }
}
