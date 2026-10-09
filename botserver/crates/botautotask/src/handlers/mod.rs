//! Real AutoTask API handlers (#755): wiring for classify / compile /
//! create-and-execute over the offline-capable classifiers, the BASIC-only
//! execution pipeline (#754) and the Drive persistence facade.

pub(crate) use crate::api::{ AutoTaskApi, ClassifyIntentRequest, ClassifyIntentResponse, CompileIntentRequest, CompileIntentResponse, CreateAndExecuteRequest, CreateAndExecuteResponse, DecisionRequest, ExecutePlanRequest, ExecutePlanResponse, PlanStepResponse, ResourceEstimateResponse, RiskResponse, TaskActionResponse, };
pub(crate) use crate::execution::script_for;
pub(crate) use crate::intent_classifier::{ClassifiedEntities, IntentClassifier, IntentType};
pub(crate) use crate::intent_compiler::IntentCompiler;
pub(crate) use crate::templates::match_shipped_template;
pub(crate) use crate::ClassifiedIntent;
pub(crate) use crate::types::{BotInfo, DbPool};
pub(crate) use axum::{ extract::{Path, Query, State}, response::IntoResponse, Json, };
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_query;
pub(crate) use diesel::sql_types::{BigInt, Float8, Nullable as NullableSql, Text, Uuid as DieselUuid};
pub(crate) use log::{info, warn};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

#[derive(diesel::QueryableByName)]
struct BotIdRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
}

/// Persisted classification row backing an executable plan.
#[derive(diesel::QueryableByName)]
struct PlanRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    bot_id: Uuid,
    #[diesel(sql_type = Text)]
    original_text: String,
    #[diesel(sql_type = Text)]
    intent_type: String,
    #[diesel(sql_type = Float8)]
    confidence: f64,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    suggested_name: Option<String>,
}

#[derive(diesel::QueryableByName)]
struct TaskIdRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
}

#[derive(diesel::QueryableByName)]
struct BotBranchRow {
    #[diesel(sql_type = NullableSql<DieselUuid>)]
    id: Option<Uuid>,
}

/// AutoTask items = rows of `auto_tasks`. The ✏️ action resolves the item's
/// `.bas` through /api/autotask/sources (the repository), so the row carries
/// the script path for display only.
#[derive(diesel::QueryableByName)]
struct AutoTaskRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    bot_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    title: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    intent: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    basic_program: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    updated_at: chrono::DateTime<chrono::Utc>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub(crate) use part1::*;
pub use part2::*;
pub use part3::*;
pub(crate) use part4::*;
pub use part5::*;
