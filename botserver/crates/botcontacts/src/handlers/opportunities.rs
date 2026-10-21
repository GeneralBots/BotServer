use axum::{
    http::HeaderMap,

    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::audit;
use crate::models::*;
use crate::requests::*;
use crate::schema::crm_deals;
use crate::CrateState;

fn db_err<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
}

fn default_bot_id(state: &CrateState) -> Uuid {
    use crate::schema::bots::dsl::{bots, id, is_default_for_branch};

    let Ok(mut conn) = state.db_pool.get() else {
        return Uuid::nil();
    };
    bots
        .filter(is_default_for_branch.eq(true))
        .select(id)
        .first::<Uuid>(&mut conn)
        .unwrap_or(Uuid::nil())
}

pub async fn create_opportunity(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<CreateOpportunityRequest>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());

    // #1441 C4 — an unnamed opportunity cannot be told apart in the grid.
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name is required".to_string()));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();

    let expected_close = req
        .expected_close_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let stage = req.stage.unwrap_or_else(|| "qualification".to_string());
    let probability = stage_probability(&stage);

    let opportunity = CrmDeal {
        id,
        org_id: branch_id,
        bot_id: default_bot_id(&state),
        branch_id,
        lead_id: req.lead_id,
        account_id: req.account_id,
        contact_id: req.contact_id,
        am_id: None,
        title: None,
        name: Some(name),
        description: req.description,
        value: req.value,
        currency: req.currency.or(Some("USD".to_string())),
        stage_id: None,
        stage: Some(stage),
        probability: Some(probability),
        source: None,
        segment_id: None,
        department_id: None,
        expected_close_date: expected_close,
        actual_close_date: None,
        period: None,
        deal_date: None,
        won: None,
        owner_id: None,
        lost_reason: None,
        closed_at: None,
        notes: None,
        tags: None,
        custom_fields: Some(serde_json::json!({})),
        created_at: Some(now),
        updated_at: Some(now),
    };

    diesel::insert_into(crm_deals::table)
        .values(&opportunity)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "opportunity",
            entity_id: Some(id),
            action: "create",
            before: None,
            after: Some(serde_json::to_value(&opportunity).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );
    Ok(Json(opportunity))
}

pub async fn list_opportunities(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    use axum::response::IntoResponse;
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    // Built twice (count + page) because boxed diesel queries are consumed
    // on execution; the closure keeps the filter list in one place (#1441 P2).
    let make_q = || {
        let mut q = crm_deals::table
            .filter(crm_deals::branch_id.eq(branch_id))
            .into_boxed();
        if let Some(stage) = &query.stage {
            q = q.filter(crm_deals::stage.eq(stage.clone()));
        }
        if let Some(search) = &query.search {
            let pattern = format!("%{search}%");
            q = q.filter(crm_deals::name.ilike(pattern));
        }
        if let Some(department_id) = query.department_id {
            q = q.filter(crm_deals::department_id.eq(department_id));
        }
        if let Some(source) = &query.source {
            q = q.filter(crm_deals::source.eq(source.clone()));
        }
        q
    };

    // #1441 P2 — X-Total-Count lets the UI paginate without a second probe.
    let total: i64 = make_q()
        .count()
        .get_result(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Count error: {e}")))?;

    let opportunities: Vec<CrmDeal> = make_q()
        .order(crm_deals::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(([("X-Total-Count", total.to_string())], Json(opportunities)).into_response())
}

pub async fn get_opportunity(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| state.get_bot_context());

    let opp: CrmDeal = crm_deals::table
        .filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Opportunity not found".to_string()))?;

    Ok(Json(opp))
}

/// `PUT /api/crm/opportunities/:id` — stage/value edits in one statement,
/// audited with the before/after snapshot.
pub async fn update_opportunity(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOpportunityRequest>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| state.get_bot_context());

    let before: CrmDeal = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Opportunity not found".to_string()))?;

    let probability = req.stage.as_deref().map(stage_probability);
    let expected_close = req
        .expected_close_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    diesel::update(
        crm_deals::table
            .filter(crm_deals::id.eq(id))
            .filter(crm_deals::branch_id.eq(branch_id)),
    )
    .set((
        req.name.clone().map(|v| crm_deals::name.eq(v)),
        req.value.map(|v| crm_deals::value.eq(v)),
        req.currency.clone().map(|v| crm_deals::currency.eq(v)),
        req.stage.clone().map(|v| crm_deals::stage.eq(v)),
        probability.map(|v| crm_deals::probability.eq(v)),
        expected_close.map(|v| crm_deals::expected_close_date.eq(v)),
        req.description.clone().map(|v| crm_deals::description.eq(v)),
        req.source.clone().map(|v| crm_deals::source.eq(v)),
        req.owner_id.map(|v| crm_deals::owner_id.eq(v)),
        crm_deals::updated_at.eq(Utc::now()),
    ))
    .execute(&mut conn)
    .map_err(db_err)?;

    let after: CrmDeal = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .unwrap_or_else(|_| before.clone());
    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "opportunity",
            entity_id: Some(id),
            action: "update",
            before: Some(serde_json::to_value(&before).unwrap_or(serde_json::Value::Null)),
            after: Some(serde_json::to_value(&after).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );
    Ok(Json(after))
}

pub async fn close_opportunity(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<CloseOpportunityRequest>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| state.get_bot_context());

    let now = Utc::now();
    let close_date = req.actual_close_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| now.date_naive());

    let stage = if req.won { "won" } else { "lost" };
    let probability = if req.won { 100 } else { 0 };

    let before: CrmDeal = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Opportunity not found".to_string()))?;

    diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)))
        .set((
            crm_deals::won.eq(Some(req.won)),
            crm_deals::stage.eq(stage),
            crm_deals::probability.eq(probability),
            crm_deals::actual_close_date.eq(Some(close_date)),
            req.lost_reason.clone().map(|v| crm_deals::lost_reason.eq(v)),
            crm_deals::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let after: CrmDeal = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .unwrap_or_else(|_| before.clone());
    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "opportunity",
            entity_id: Some(id),
            action: if req.won { "close_won" } else { "close_lost" },
            before: Some(serde_json::to_value(&before).unwrap_or(serde_json::Value::Null)),
            after: Some(serde_json::to_value(&after).unwrap_or(serde_json::Value::Null)),
            detail: req.lost_reason.clone().map(|r| serde_json::json!({ "lost_reason": r })),
        },
    );
    Ok(Json(after))
}

pub async fn delete_opportunity(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| state.get_bot_context());

    let before: Option<CrmDeal> = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .ok();

    // #1441 C5 — destructive action restricted to the record owner or an admin.
    crate::authz::ensure_can_modify(&mut conn, &headers, before.as_ref().and_then(|d| d.owner_id))?;

    diesel::delete(crm_deals::table.filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    if let Some(row) = before {
        audit::record(
            &state,
            audit::AuditRecord {
                headers: &headers,
                branch_id,
                entity: "opportunity",
                entity_id: Some(id),
                action: "delete",
                before: Some(serde_json::to_value(&row).unwrap_or(serde_json::Value::Null)),
                after: None,
                detail: None,
            },
        );
    }
    Ok(StatusCode::NO_CONTENT)
}
