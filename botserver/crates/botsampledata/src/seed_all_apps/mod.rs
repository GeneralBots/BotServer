//! Demo-data seeding for the remaining suite apps.
//!
//! The base `db.rs` seeds people, CRM, tickets, billing, tasks, calendar,
//! research, compliance, goals, workspaces, social, marketing, o365 and drive.
//! This module extends coverage to the rest of the harvested app surface so the
//! chat command palette and `api.exec` return real data for every application:
//! products, analytics/dashboards, meet + minutes, learn, project, canvas,
//! attendant, plan/OKRs, database, integrations, sources, monitoring.
//!
//! Two kinds of tables are seeded:
//!   * Central-schema tables (products, meeting_rooms, canvases, ...) that are
//!     always present — seeded unconditionally.
//!   * Crate-owned tables (sales_deals, hr_employees, brazil_nfe, ...) that are
//!     created lazily when their owning crate initializes — seeded only when the
//!     table exists (guarded by `to_regclass`), so a missing table never aborts
//!     the rest of the demo data.
//!
//! Every statement is idempotent (guarded by a count check), matching the
//! existing `botsampledata` contract, and carries the NOT-NULL columns the real
//! schema requires (`org_id`, `bot_id`, `branch_id` as applicable).

pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_query;
pub(crate) use diesel::sql_types::{Integer, Nullable, Text, Uuid as SqlUuid};
pub(crate) use uuid::Uuid;
pub(crate) use super::db::Scopes;

mod part1;
mod part2;

pub use part1::*;
use part2::*;
