//! #1441 — deal reads/creation and the branch summary stats. State-changing
//! deal and activity handlers live in `deals_mutate.rs` / `activities.rs` and
//! the stage catalog in `crate::stages`, so every file stays inside the
//! 450-line budget.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::audit;
use crate::models::*;
use crate::requests::*;
use crate::schema::{crm_accounts, crm_contacts, crm_deals};
use crate::CrateState;

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

pub async fn list_deals(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<CrmDeal>>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .into_boxed();

    if let Some(stage) = query.stage {
        q = q.filter(crm_deals::stage.eq(stage));
    }

    if let Some(search) = query.search {
        let pattern = format!("%{search}%");
        q = q.filter(crm_deals::title.ilike(pattern.clone()).or(crm_deals::name.ilike(pattern)));
    }

    if let Some(department_id) = query.department_id {
        q = q.filter(crm_deals::department_id.eq(department_id));
    }

    if let Some(source) = query.source {
        q = q.filter(crm_deals::source.eq(source));
    }

    if let Some(owner_id) = query.owner_id {
        q = q.filter(crm_deals::owner_id.eq(owner_id));
    }

    if let Some(status) = query.status {
        match status.as_str() {
            "open" => { q = q.filter(crm_deals::closed_at.is_null()); }
            "closed" => { q = q.filter(crm_deals::closed_at.is_not_null()); }
            "won" => { q = q.filter(crm_deals::won.eq(true)); }
            "lost" => { q = q.filter(crm_deals::won.eq(false)); }
            _ => {}
        }
    }

    let deals: Vec<CrmDeal> = q
        .order(crm_deals::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(deals))
}

pub async fn create_deal(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<CreateDealRequest>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());

    // #1441 C4 — required-field validation server-side: an unnamed deal is
    // unusable in every grid and cannot be searched.
    let title = req
        .title
        .clone()
        .or_else(|| req.name.clone())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    let Some(title) = title else {
        return Err((
            StatusCode::BAD_REQUEST,
            "title (or name) is required".to_string(),
        ));
    };

    let id = Uuid::new_v4();
    let now = Utc::now();

    let expected_close = req
        .expected_close_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let stage = req.stage.unwrap_or_else(|| "new".to_string());
    let probability = stage_probability(&stage);

    let deal = CrmDeal {
        id,
        org_id: branch_id,
        bot_id: default_bot_id(&state),
        branch_id,
        contact_id: req.contact_id,
        account_id: req.account_id,
        am_id: None,
        lead_id: None,
        owner_id: req.owner_id,
        title: Some(title),
        name: req.name.unwrap_or_default(),
        description: req.description,
        value: req.value,
        currency: req.currency.or(Some("USD".to_string())),
        stage_id: None,
        stage: Some(stage.clone()),
        probability: Some(probability),
        source: req.source,
        segment_id: None,
        department_id: req.department_id,
        expected_close_date: expected_close,
        actual_close_date: None,
        period: None,
        deal_date: None,
        lost_reason: None,
        won: if stage == "won" { Some(true) } else if stage == "lost" { Some(false) } else { None },
        tags: req.tags,
        custom_fields: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        closed_at: if stage == "won" || stage == "lost" { Some(now) } else { None },
        notes: req.notes,
    };

    diesel::insert_into(crm_deals::table)
        .values(&deal)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert deal error: {e}")))?;

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "deal",
            entity_id: Some(id),
            action: "create",
            before: None,
            after: Some(serde_json::to_value(&deal).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );
    Ok(Json(deal))
}

pub async fn get_deal(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| state.get_bot_context());

    let deal: CrmDeal = crm_deals::table
        .filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Deal not found".to_string()))?;

    Ok(Json(deal))
}





pub async fn get_crm_stats(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Result<Json<CrmStats>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| state.get_bot_context());

    let total_contacts: i64 = crm_contacts::table
        .filter(crm_contacts::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let total_accounts: i64 = crm_accounts::table
        .filter(crm_accounts::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let total_leads: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::closed_at.is_null())
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let total_opportunities: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::won.is_null())
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let won_this_month: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::won.eq(Some(true)))
        .filter(crm_deals::closed_at.ge(Utc::now() - chrono::Duration::days(31)))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    // #1441 C7 — the summary card reported 0.0 regardless of the real pipeline.
    let pipeline_value: Option<f64> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::closed_at.is_null())
        .select(diesel::dsl::sum(crm_deals::value))
        .get_result(&mut conn)
        .unwrap_or(None);

    let total_campaigns: i64 = crate::schema::marketing_campaigns::table
        .filter(crate::schema::marketing_campaigns::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let stats = CrmStats {
        total_contacts,
        total_accounts,
        total_leads,
        total_opportunities,
        total_campaigns,
        pipeline_value: pipeline_value.unwrap_or(0.0),
        won_this_month,
        conversion_rate: if total_leads > 0 {
            (won_this_month as f64 / total_leads as f64) * 100.0
        } else {
            0.0
        },
    };

    Ok(Json(stats))
}
