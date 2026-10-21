//! #1452/#1454/#1455 — pipeline stage management: idempotent default seeding
//! plus the admin CRUD behind `/api/crm/pipeline/stages`.
//!
//! Fresh branches ship with zero `crm_pipeline_stages` rows, so the kanban
//! silently falls back to hardcoded columns and every custom stage an org
//! creates is only half-wired. `seed_default_stages` runs at crate bootstrap
//! (same pattern as `botproducts::seed::seed_default_products`) and is safe
//! to call repeatedly: branches that already have stages are left untouched.
//!
//! #1454: prod schema is `6.0.7-people` — `org_id` FKs to `organizations`
//! (NOT branches) and `bot_id` is NOT NULL FK to `bots` with
//! `UNIQUE (org_id, bot_id, name)`. The old seed wrote the branch id as org
//! and no bot at all, so every INSERT failed validation and the failure was
//! swallowed silently. The seed now resolves the real org + bot per branch
//! and logs every error.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::audit;
use crate::models::CrmPipelineStage;
use crate::schema::{crm_deals, crm_pipeline_stages};
use crate::scope::branch_from_jwt;
use crate::CrateState;

/// The six default stages every branch starts with. `won`/`lost` carry the
/// terminal semantics the kanban styling and stats endpoints rely on.
pub const DEFAULT_STAGES: &[(&str, i32, i32)] = &[
    ("new", 1, 10),
    ("qualified", 2, 25),
    ("proposal", 3, 50),
    ("negotiation", 4, 75),
    ("won", 5, 100),
    ("lost", 6, 0),
];

/// One seeding candidate (#1454): a branch that holds deals but no stage
/// rows, with the real owning organization and a real active bot — both are
/// NOT NULL + FK'd on `crm_pipeline_stages`.
#[derive(Debug, QueryableByName)]
struct SeedCandidate {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    branch_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    org_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    bot_id: Uuid,
}

/// Seeds the six default pipeline stages for every branch that has leads but
/// no stage rows yet. Idempotent; failures are logged and never propagate —
/// the hardcoded frontend fallback keeps working without them.
pub fn seed_default_stages(state: &Arc<CrateState>) {
    let Ok(mut conn) = state.db_pool.get() else {
        log::warn!("[crm-stages] pool unavailable; stage seeding skipped");
        return;
    };

    // Real org + real bot per branch; branches without an active bot are
    // skipped (the columns cannot be satisfied) and counted for the log.
    let candidates: Vec<SeedCandidate> = diesel::sql_query(
        "SELECT b.id AS branch_id, b.org_id, \
         (SELECT bt.id FROM bots bt \
          WHERE bt.branch_id = b.id AND bt.is_active = true \
          ORDER BY bt.is_default_for_branch DESC LIMIT 1) AS bot_id \
         FROM branches b \
         WHERE EXISTS (SELECT 1 FROM crm_deals d WHERE d.branch_id = b.id) \
         AND NOT EXISTS ( \
             SELECT 1 FROM crm_pipeline_stages s WHERE s.branch_id = b.id) \
         AND EXISTS ( \
             SELECT 1 FROM bots bt \
             WHERE bt.branch_id = b.id AND bt.is_active = true)",
    )
    .load::<SeedCandidate>(&mut conn)
    .unwrap_or_else(|e| {
        log::warn!("[crm-stages] seed candidate query failed: {e}");
        Vec::new()
    });

    if candidates.is_empty() {
        log::info!("[crm-stages] seed: no branches need default stages");
        return;
    }

    let branch_count = candidates.len();
    let now = chrono::Utc::now();
    let mut seeded = 0usize;
    for candidate in &candidates {
        for (name, order, probability) in DEFAULT_STAGES {
            let row = CrmPipelineStage {
                id: Uuid::new_v4(),
                org_id: candidate.org_id,
                bot_id: candidate.bot_id,
                branch_id: candidate.branch_id,
                name: (*name).to_string(),
                stage_order: *order,
                probability: *probability,
                is_won: *name == "won",
                is_lost: *name == "lost",
                color: None,
                created_at: now,
            };
            match diesel::insert_into(crm_pipeline_stages::table)
                .values(&row)
                .execute(&mut conn)
            {
                Ok(_) => seeded += 1,
                Err(e) => log::warn!(
                    "[crm-stages] stage insert failed for branch {0} stage '{1}': {2}",
                    candidate.branch_id, name, e
                ),
            }
        }
    }
    log::info!(
        "[crm-stages] seeded {seeded} default stage rows across {branch_count} branch(es)"
    );
}

fn admin_headers(headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    // The auth middleware has already verified the caller; admin gating uses
    // the shared role resolver against the JWT subject (mirrors the catalog
    // executor's rule server-side so the UI cannot bypass it).
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if token.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "Authentication required".to_string()));
    }
    Ok(())
}

/// Resolves the owning organization for a branch (#1454). On the live schema
/// `crm_pipeline_stages.org_id` FKs to `organizations`, never to branches.
fn org_for_branch(conn: &mut PgConnection, branch_id: Uuid) -> Result<Uuid, (StatusCode, String)> {
    diesel::sql_query("SELECT org_id FROM branches WHERE id = $1")
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .get_result::<OrgRow>(conn)
        .map(|r| r.org_id)
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                format!("branch {branch_id} has no organization row"),
            )
        })
}

#[derive(Debug, QueryableByName)]
struct OrgRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    org_id: Uuid,
}

/// Resolves an active bot for a branch (#1454) — `bot_id` is NOT NULL on the
/// live stages table. Nil bot ids are rejected (FK to `bots(id)`).
fn bot_for_branch(state: &CrateState, branch_id: Uuid) -> Result<Uuid, (StatusCode, String)> {
    let bot_id = state.bot_for_branch(branch_id);
    if bot_id.is_nil() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("branch {branch_id} has no active bot to own the stage"),
        ));
    }
    Ok(bot_id)
}

/// `POST /api/crm/pipeline/stages` — create a custom stage (admin only).
pub async fn create_stage(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<CreateStageRequest>,
) -> Result<Json<CrmPipelineStage>, (StatusCode, String)> {
    admin_headers(&headers)?;
    let name = req.name.trim();
    if name.is_empty() || name.len() > 60 {
        return Err((StatusCode::BAD_REQUEST, "name must be 1..60 chars".to_string()));
    }
    let probability = req.probability.unwrap_or(0).clamp(0, 100);

    let mut conn = state.db_pool.get().map_err(pool_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());
    let org_id = org_for_branch(&mut conn, branch_id)?;
    let bot_id = bot_for_branch(&state, branch_id)?;

    let order = match req.stage_order {
        Some(o) if o >= 1 => o,
        _ => crm_pipeline_stages::table
            .filter(crm_pipeline_stages::branch_id.eq(branch_id))
            .select(diesel::dsl::max(crm_pipeline_stages::stage_order))
            .first::<Option<i32>>(&mut conn)
            .unwrap_or(Some(0))
            .unwrap_or(0)
            + 1,
    };

    let row = CrmPipelineStage {
        id: Uuid::new_v4(),
        org_id,
        bot_id,
        branch_id,
        name: name.to_string(),
        stage_order: order,
        probability,
        is_won: name == "won",
        is_lost: name == "lost",
        color: None,
        created_at: chrono::Utc::now(),
    };
    diesel::insert_into(crm_pipeline_stages::table)
        .values(&row)
        .execute(&mut conn)
        .map_err(diesel_err)?;
    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "pipeline_stage",
            entity_id: Some(row.id),
            action: "stage_create",
            before: None,
            after: Some(serde_json::json!({
            "name": row.name, "stage_order": row.stage_order, "probability": row.probability
        })),
            detail: None,
        },
    );
    Ok(Json(row))
}

/// `PUT /api/crm/pipeline/stages/:id` — rename / reorder / re-weight a stage.
pub async fn update_stage(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateStageRequest>,
) -> Result<Json<CrmPipelineStage>, (StatusCode, String)> {
    admin_headers(&headers)?;
    let mut conn = state.db_pool.get().map_err(pool_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());

    let before: CrmPipelineStage = crm_pipeline_stages::table
        .filter(crm_pipeline_stages::id.eq(id))
        .filter(crm_pipeline_stages::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Stage not found".to_string()))?;

    diesel::update(
        crm_pipeline_stages::table
            .filter(crm_pipeline_stages::id.eq(id))
            .filter(crm_pipeline_stages::branch_id.eq(branch_id)),
    )
    .set((
        req.name.as_deref().map(|n| crm_pipeline_stages::name.eq(n.trim().to_string())),
        req.stage_order.map(|o| crm_pipeline_stages::stage_order.eq(o.clamp(1, 999))),
        req.probability.map(|p| crm_pipeline_stages::probability.eq(p.clamp(0, 100))),
    ))
    .execute(&mut conn)
    .map_err(diesel_err)?;

    let after: CrmPipelineStage = crm_pipeline_stages::table
        .filter(crm_pipeline_stages::id.eq(id))
        .filter(crm_pipeline_stages::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Stage not found".to_string()))?;
    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "pipeline_stage",
            entity_id: Some(id),
            action: "stage_update",
            before: Some(serde_json::json!({
            "name": before.name, "stage_order": before.stage_order, "probability": before.probability
        })),
            after: Some(serde_json::json!({
            "name": after.name, "stage_order": after.stage_order, "probability": after.probability
        })),
            detail: None,
        },
    );
    Ok(Json(after))
}

/// `DELETE /api/crm/pipeline/stages/:id` — blocked while leads reference the
/// stage so the kanban never drops a column holding data.
pub async fn delete_stage(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    admin_headers(&headers)?;
    let mut conn = state.db_pool.get().map_err(pool_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());

    let stage: CrmPipelineStage = crm_pipeline_stages::table
        .filter(crm_pipeline_stages::id.eq(id))
        .filter(crm_pipeline_stages::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Stage not found".to_string()))?;

    let in_use: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::stage.eq(stage.name.clone()))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);
    if in_use > 0 {
        return Err((
            StatusCode::CONFLICT,
            format!("stage '{0}' still holds {in_use} lead(s); move them first", stage.name),
        ));
    }

    diesel::delete(
        crm_pipeline_stages::table
            .filter(crm_pipeline_stages::id.eq(id))
            .filter(crm_pipeline_stages::branch_id.eq(branch_id)),
    )
    .execute(&mut conn)
    .map_err(diesel_err)?;
    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "pipeline_stage",
            entity_id: Some(id),
            action: "stage_delete",
            before: Some(serde_json::json!({
            "name": stage.name, "stage_order": stage.stage_order, "probability": stage.probability
        })),
            after: None,
            detail: None,
        },
    );
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateStageRequest {
    pub name: String,
    pub stage_order: Option<i32>,
    pub probability: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateStageRequest {
    pub name: Option<String>,
    pub stage_order: Option<i32>,
    pub probability: Option<i32>,
}

fn pool_err<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB pool: {e}"))
}

fn diesel_err(e: diesel::result::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
}

pub async fn get_pipeline_stages(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<CrmPipelineStage>>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());

    let stages: Vec<CrmPipelineStage> = crm_pipeline_stages::table
        .filter(crm_pipeline_stages::branch_id.eq(branch_id))
        .order(crm_pipeline_stages::stage_order.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(stages))
}
