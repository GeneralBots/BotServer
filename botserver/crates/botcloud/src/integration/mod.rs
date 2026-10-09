//! SaaS Integration → CRM, ERP (GL) and Subscriptions.
//!
//! Connects the SaaS checkout flow with:
//! - **CRM**: creates contact and deal (opportunity) in pipeline
//! - **ERP (GL)**: posts entry to general ledger when invoice is paid
//! - **Subscription**: creates recurring record in `billing_recurring`
//! - **Signup**: creates tenant, org, branch, bot, MinIO bucket, trial subscription, workspace

pub(crate) use chrono::{Utc, Duration};
pub(crate) use diesel::deserialize::QueryableByName;
pub(crate) use diesel::sql_types::{Date, Nullable, Numeric, Text, Timestamptz, Uuid as DieselUuid};
pub(crate) use diesel::RunQueryDsl;
pub(crate) use diesel::PgConnection;
pub(crate) use botsecurity::command_guard::SafeCommand;
pub(crate) use std::io::Write;
pub(crate) use std::path::Path;
pub(crate) use uuid::Uuid;

/// Helper struct to read Uuid from raw SQL queries.
#[derive(QueryableByName, Debug)]
struct IdRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
}

use crate::{DbPool, SaasConfig};

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
