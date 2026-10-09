//! Reform #1500/#1504 — Vibe bootstrap for the branch's default bot.
//!
//! Every branch owns a default bot; the reform makes that bot a **Vibe
//! project** so people start vibing the base bot immediately:
//!
//! - `ensure_branch_default_project` creates the `bot`-kind, git-mode project
//!   row idempotently (safe to call from signup, Drive monitor scans and the
//!   boot backfill) and **adopts** the branch's existing `bots` row instead of
//!   duplicating it — the adopted row keeps its identity, LLM config and
//!   channels; only `payload.bot_id` links project ↔ bot.
//! - `ensure_bot_rows` guarantees the two-environment model: the PROD bot
//!   (adopted or created) and the `{bot}-test` TEST twin (always created,
//!   `origin='vibe'`), whose database is the project's `_dev` twin per #1386.
//!
//! The boot backfill lives in `bootstrap_backfill.rs`; both share the
//! `pub(crate)` SQL helpers in this module.

pub(crate) use diesel::connection::SimpleConnection;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::OptionalExtension;
pub(crate) use uuid::Uuid;
pub(crate) use crate::projects::Project;

/// Fallback `vibe_projects` DDL for cold-start bootstraps (the registry's
/// schema ensure runs at boot; this keeps the bootstrap self-sufficient when
/// it fires before the first router mount, e.g. signup hook on a fresh DB).
pub(crate) const VIBE_PROJECTS_FALLBACK_SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS vibe_projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    branch_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    name VARCHAR(255) NOT NULL,
    project_type VARCHAR(32) NOT NULL DEFAULT 'bot',
    repository VARCHAR(255) NOT NULL DEFAULT '',
    framework VARCHAR(64) NOT NULL DEFAULT '',
    custom_domain VARCHAR(255) NOT NULL DEFAULT '',
    source_control VARCHAR(16) NOT NULL DEFAULT 'git',
    status VARCHAR(32) NOT NULL DEFAULT 'ready',
    environment VARCHAR(16) NOT NULL DEFAULT 'development',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT vibe_projects_branch_name_unique UNIQUE (branch_id, name)
);
CREATE INDEX IF NOT EXISTS idx_vibe_projects_branch ON vibe_projects(branch_id);
CREATE INDEX IF NOT EXISTS idx_vibe_projects_type ON vibe_projects(project_type);
";

/// Identity of the `bots` row(s) a bot-kind project owns/adopts.
#[derive(Debug)]
pub struct BotRowRef {
    pub id: Uuid,
    pub org_id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
}

#[derive(diesel::QueryableByName)]
struct BotIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
}

#[derive(diesel::QueryableByName)]
struct ProjectIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
}

mod slug;
mod bot_ref;
mod insert_find;
mod ensure;
mod default_project;
mod pool;
mod tests;

pub use slug::*;
use insert_find::*;
pub use ensure::*;
pub(crate) use default_project::*;
pub use pool::*;
