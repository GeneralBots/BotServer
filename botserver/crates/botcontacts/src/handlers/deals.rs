use axum::{
    http::HeaderMap,

    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
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

fn get_bot_context(state: &CrateState) -> Uuid {
    state.get_bot_context()
}

fn default_bot_id(state: &CrateState) -> Uuid {
    use crate::schema::bots::dsl::{bots, id, is_default_for_branch};
    use diesel::prelude::*;

    let Ok(mut conn) = state.db_pool.get() else {
        return Uuid::nil();
    };
    bots
        .filter(is_default_for_branch.eq(true))
        .select(id)
        .first::<Uuid>(&mut conn)
        .unwrap_or(Uuid::nil())
}

pub async fn create_lead_form(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<CreateLeadForm>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let effective_branch_id = branch_id;
    let id = Uuid::new_v4();
    let now = Utc::now();

    let title = req.title.or_else(|| {
        match (req.first_name.as_deref(), req.last_name.as_deref()) {
            (Some(first), Some(last)) => Some(format!("{first} {last}")),
            (Some(first), None) => Some(first.to_string()),
            (None, Some(last)) => Some(last.to_string()),
            (None, None) => Some("New Lead".to_string()),
        }
    }).unwrap_or_else(|| "New Lead".to_string());

    // #1441 A1/A2 — the lead form captures the prospect's contact data, so it
    // must never be discarded: find-or-create the contact (dedupe by email,
    // branch-scoped) and the account (dedupe by company name), then link both
    // on the deal. Returns the created/attached ids on the lead.
    let contact_id = match req.email.as_deref().map(str::trim).filter(|e| !e.is_empty()) {
        Some(email) => {
            let existing: Option<CrmContact> = crm_contacts::table
                .filter(crm_contacts::branch_id.eq(effective_branch_id))
                // ILIKE without wildcards = case-insensitive exact match (A2 dedupe)
                .filter(crm_contacts::email.ilike(email))
                .first::<CrmContact>(&mut conn)
                .ok();
            match existing {
                Some(c) => Some(c.id),
                None => {
                    let cid = Uuid::new_v4();
                    let contact = CrmContact {
                        id: cid,
                        org_id: effective_branch_id,
                        bot_id: state.bot_for_branch(effective_branch_id),
                        branch_id: effective_branch_id,
                        first_name: req.first_name.clone(),
                        last_name: req.last_name.clone(),
                        email: Some(email.to_string()),
                        phone: req.phone.clone(),
                        mobile: None,
                        company: req.company.clone(),
                        job_title: req.job_title.clone(),
                        source: req.source.clone(),
                        status: Some("lead".to_string()),
                        tags: None,
                        custom_fields: None,
                        notes: None,
                        owner_id: None,
                        pass_hash: None,
                        created_at: now,
                        updated_at: now,
                        address_line1: None,
                        address_line2: None,
                        city: None,
                        state: None,
                        postal_code: None,
                        country: None,
                    };
                    diesel::insert_into(crm_contacts::table)
                        .values(&contact)
                        .execute(&mut conn)
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert contact error: {e}")))?;
                    Some(cid)
                }
            }
        }
        None => None,
    };

    let account_id = match req.company.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        Some(company) => {
            let existing: Option<CrmAccount> = crm_accounts::table
                .filter(crm_accounts::branch_id.eq(effective_branch_id))
                .filter(crm_accounts::name.ilike(company))
                .first::<CrmAccount>(&mut conn)
                .ok();
            match existing {
                Some(a) => Some(a.id),
                None => {
                    let aid = Uuid::new_v4();
                    let account = CrmAccount {
                        id: aid,
                        org_id: effective_branch_id,
                        bot_id: state.bot_for_branch(effective_branch_id),
                        branch_id: effective_branch_id,
                        name: company.to_string(),
                        industry: None,
                        website: None,
                        phone: None,
                        email: None,
                        owner_id: None,
                        created_at: now,
                        updated_at: now,
                        employees_count: None,
                        annual_revenue: None,
                        address_line1: None,
                        address_line2: None,
                        city: None,
                        state: None,
                        postal_code: None,
                        country: None,
                        description: None,
                        tags: Vec::new(),
                        custom_fields: serde_json::json!({}),
                    };
                    diesel::insert_into(crm_accounts::table)
                        .values(&account)
                        .execute(&mut conn)
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert account error: {e}")))?;
                    Some(aid)
                }
            }
        }
        None => None,
    };

    let lead = CrmDeal {
        id,
        org_id: effective_branch_id,
        bot_id: default_bot_id(&state),
        branch_id: effective_branch_id,
        contact_id,
        account_id,
        am_id: None,
        lead_id: None,
        title: Some(title),
        name: String::new(),
        description: req.description,
        value: req.value,
        currency: Some(req.currency.unwrap_or_else(|| "USD".to_string())),
        stage_id: None,
        stage: Some("new".to_string()),
        probability: Some(10),
        source: req.source.clone(),
        segment_id: None,
        department_id: None,
        expected_close_date: None,
        actual_close_date: None,
        period: None,
        deal_date: None,
        owner_id: None,
        lost_reason: req.lost_reason,
        won: None,
        tags: None,
        custom_fields: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        closed_at: None,
        notes: None,
    };

    diesel::insert_into(crm_deals::table)
        .values(&lead)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert lead error: {e}")))?;

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id: effective_branch_id,
            entity: "lead",
            entity_id: Some(id),
            action: "create",
            before: None,
            after: Some(serde_json::to_value(&lead).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );

    Ok(Json(lead))
}

pub async fn create_lead(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let expected_close = req.expected_close_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let lead = CrmDeal {
        id,
        org_id: branch_id,
        bot_id: default_bot_id(&state),
        branch_id,
        contact_id: req.contact_id,
        account_id: req.account_id,
        am_id: None,
        lead_id: None,
        title: Some(req.title),
        name: String::new(),
        description: req.description,
        value: req.value,
        currency: req.currency.or(Some("USD".to_string())),
        stage_id: None,
        stage: Some("new".to_string()),
    probability: Some(10),
        source: req.source,
        segment_id: None,
        department_id: None,
        expected_close_date: expected_close,
        actual_close_date: None,
        period: None,
        deal_date: None,
        owner_id: None,
        lost_reason: None,
        won: None,
        tags: None,
        custom_fields: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        closed_at: None,
        notes: None,
    };

    diesel::insert_into(crm_deals::table)
        .values(&lead)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(lead))
}

pub async fn list_leads(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
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
            q = q.filter(crm_deals::title.ilike(pattern));
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

    let leads: Vec<CrmDeal> = make_q()
        .order(crm_deals::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(([("X-Total-Count", total.to_string())], Json(leads)))
}

pub async fn get_lead(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    let lead: CrmDeal = crm_deals::table
        .filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Lead not found".to_string()))?;

    Ok(Json(lead))
}

pub async fn update_lead_stage(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<LeadStageQuery>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();
    let stage = query.stage;
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    let old_stage: Option<Option<String>> = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .select(crm_deals::stage)
        .first(&mut conn)
        .ok();

    let old_stage_str = old_stage.flatten();
    let probability = stage_probability(&stage);

    diesel::update(
        crm_deals::table
            .filter(crm_deals::id.eq(id))
            .filter(crm_deals::branch_id.eq(branch_id)),
    )
    .set((
        crm_deals::stage.eq(&stage),
        crm_deals::probability.eq(probability),
        crm_deals::updated_at.eq(now),
    ))
    .execute(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    if stage == "won" || stage == "lost" || stage == "converted" {
        diesel::update(
            crm_deals::table
                .filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)),
        )
        .set(crm_deals::closed_at.eq(Some(now)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(old) = old_stage_str {
        if old != stage {
            (state.trigger_deal_stage_change)(&mut conn, id, &old, &stage, branch_id);
            audit::record(
                &state,
                audit::AuditRecord {
                    headers: &headers,
                    branch_id,
                    entity: "lead",
                    entity_id: Some(id),
                    action: "stage_change",
                    before: Some(serde_json::json!({ "stage": old })),
                    after: Some(serde_json::json!({ "stage": stage })),
                    detail: None,
                },
            );
        }
    }

    get_lead(State(state), headers, Path(id)).await
}
