//! Database demo-data seeding.
//!
//! All inserts are guarded so re-running is safe. Every domain is scoped to
//! the dedicated sample tenant (org/branch/bot), never to a real tenant or to
//! the nil/global scope.

pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_query;
pub(crate) use diesel::sql_types::{Integer, Nullable, Text, Uuid as SqlUuid};
pub(crate) use uuid::Uuid;
pub(crate) use crate::sample::{ensure_sample_scope, SampleScope};

/// Scopes resolved at seed time from the dedicated sample tenant so demo data
/// never lands in a real tenant. Real tenants are never queried or modified.
pub struct Scopes {
    pub org_id: Uuid,
    pub branch_id: Uuid,
    pub bot_id: Uuid,
    pub user_id: Uuid,
    pub org_str: String,
    pub branch_str: String,
    pub bot_str: String,
    pub user_str: String,
}

#[derive(diesel::QueryableByName)]
struct UuidRowNamed {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
}

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
use part2::*;
use part3::*;
use part4::*;
