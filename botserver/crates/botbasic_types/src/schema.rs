pub mod tables {
    // Canonical definition lives in `botschema` (#1497).
    pub use botschema::bots;

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

    // Canonical definition lives in `botschema` (#1497). The local copy carried
    // the `6.5.15.1` shape (`bot_id`/`key`/`value`/`created_at`/`updated_at`),
    // none of which the database has; it stores `source_bot_id`, `target_bot_id`,
    // `memory_key`, `memory_value`, `shared_at`.
    pub use botschema::bot_shared_memory;

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

    // Canonical definition lives in `botschema` (#1497). The database stores
    // `id`/`session_id`/`tool_name`/`added_at` as `TEXT`, not `UUID`/`VARCHAR`.
    pub use botschema::session_tool_associations;

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

    // Canonical definition lives in `botschema` (#1497).
    pub use botschema::calendar_events;

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

// The org lookup joins `bots` to `organizations` on `org_id`. Both tables are
// canonical in `botschema`, which already grants that join in its own
// `allow_tables_to_appear_in_same_query!`; repeating the macro here would emit
// the impls for two foreign types and break the orphan rule (#1497).

pub use tables::*;
