pub mod tables {
    diesel::table! {
        bots (id) {
            id -> Uuid,
            branch_id -> Uuid,
            bot_id -> Uuid,
            name -> Varchar,
            slug -> Varchar,
            org_id -> Uuid,
            tenant_id -> Nullable<Uuid>,
            is_default_for_branch -> Nullable<Bool>,
            description -> Nullable<Text>,
            is_public -> Nullable<Bool>,
            is_active -> Nullable<Bool>,
            avatar_url -> Nullable<Varchar>,
            settings -> Nullable<Jsonb>,
            metadata -> Nullable<Jsonb>,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
            llm_provider -> Varchar,
            llm_config -> Jsonb,
            context_provider -> Varchar,
            context_config -> Jsonb,
            database_name -> Nullable<Varchar>,
        }
    }

    diesel::table! {
        user_sessions (id) {
            id -> Uuid,
            user_id -> Uuid,
            branch_id -> Uuid,
            bot_id -> Uuid,
            title -> Varchar,
            context_data -> Jsonb,
            current_tool -> Nullable<Varchar>,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
        }
    }

    diesel::table! {
        bot_memories (id) {
            id -> Uuid,
            branch_id -> Uuid,
            bot_id -> Uuid,
            key -> Varchar,
            value -> Text,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
        }
    }

    diesel::table! {
        bot_shared_memory (id) {
            id -> Uuid,
            bot_id -> Uuid,
            key -> Varchar,
            value -> Text,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
        }
    }

    diesel::table! {
        system_automations (id) {
            id -> Uuid,
            branch_id -> Uuid,
            bot_id -> Uuid,
            trigger_kind -> Varchar,
            trigger_data -> Jsonb,
            script_path -> Varchar,
            is_active -> Bool,
            schedule -> Nullable<Varchar>,
            param -> Varchar,
            last_triggered -> Nullable<Timestamptz>,
            created_at -> Timestamptz,
        }
    }

    diesel::table! {
        session_tool_associations (id) {
            id -> Uuid,
            session_id -> Uuid,
            tool_name -> Varchar,
            created_at -> Timestamptz,
        }
    }

    // Canonical definition lives in `botschema` (#1497).
    pub use botschema::message_history;
    diesel::table! {
        workflow_executions (id) {
            id -> Uuid,
            branch_id -> Uuid,
            bot_id -> Uuid,
            workflow_name -> Varchar,
            status -> Varchar,
            created_at -> Timestamptz,
        }
    }

    diesel::table! {
        workflow_events (id) {
            id -> Uuid,
            execution_id -> Uuid,
            event_type -> Varchar,
            payload -> Jsonb,
            created_at -> Timestamptz,
        }
    }

    diesel::table! {
        calendar_events (id) {
            id -> Uuid,
            branch_id -> Uuid,
            calendar_id -> Uuid,
            owner_id -> Uuid,
            title -> Varchar,
            description -> Nullable<Text>,
            location -> Nullable<Varchar>,
            start_time -> Timestamptz,
            end_time -> Timestamptz,
            all_day -> Bool,
            recurrence_rule -> Nullable<Text>,
            recurrence_id -> Nullable<Uuid>,
            status -> Varchar,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
        }
    }

    // Needed to turn `bots.org_id` into the tenant slug that names the
// `.gborg` Drive bucket — buckets are slug-based, not UUID-based.
diesel::table! {
    organizations (org_id) {
            org_id -> Uuid,
            name -> Varchar,
            slug -> Varchar,
            created_at -> Nullable<Timestamptz>,
            updated_at -> Nullable<Timestamptz>,
        }
    }

    diesel::table! {
        bot_configuration (id) {
            id -> Uuid,
            branch_id -> Uuid,
            bot_id -> Uuid,
            config_key -> Varchar,
            config_value -> Text,
            is_encrypted -> Bool,
            config_type -> Varchar,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
        }
    }
}

// The org lookup joins `bots` to `organizations` on `org_id`.
diesel::allow_tables_to_appear_in_same_query!(bots, organizations);

pub use tables::*;
