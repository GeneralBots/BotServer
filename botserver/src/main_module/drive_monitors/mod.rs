//! drive_monitors - extracted from bootstrap.rs
//! Processes .gborg buckets containing .gbai bot sub-directories

pub(crate) use botcore::shared::state::AppState;
pub(crate) use diesel::RunQueryDsl;
pub(crate) use log::{error, info, trace, warn};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::basic::AppStateBasicRuntime;
pub(crate) use crate::basic::keywords::table_definition::process_table_definitions;

const DEFAULT_TENANT_ID: Uuid = Uuid::from_u128(1);
const DEFAULT_ORG_ID: Uuid = Uuid::from_u128(1);
const DEFAULT_BRANCH_ID: Uuid = Uuid::from_u128(1);
const DEFAULT_BOT_ID: Uuid = Uuid::from_u128(1);

mod part1;
mod part2;
mod part3;
mod part4;

pub(crate) use part1::*;
use part2::*;
use part3::*;
pub(crate) use part4::*;
