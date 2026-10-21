//! #1441 P2 — CSV import/export for leads plus the audit-trail reader
//! (split from `bulk_csv.rs` to respect the 450-line budget; the bulk
//! action handlers stay there).

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::audit;
use crate::models::CrmDeal;
use crate::requests::{CsvImportReport, CsvLeadRow};
use crate::schema::{crm_accounts, crm_contacts, crm_deals};
use crate::scope::{branch_from_jwt, email_from_jwt};
use crate::CrateState;

fn get_bot_context(state: &CrateState) -> Uuid {
    state.get_bot_context()
}

/// #1456 — import options: `?dry_run=true` validates without writing.
#[derive(Debug, serde::Deserialize)]
pub struct ImportQuery {
    pub dry_run: Option<bool>,
}

fn db_err<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
}

// ────────────────────────────────────────────────────────────────────────────
// CSV export / import
// ────────────────────────────────────────────────────────────────────────────

fn csv_cell(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn csv_opt(value: &Option<String>) -> String {
    csv_cell(value.as_deref().unwrap_or(""))
}

fn csv_number(value: Option<f64>) -> String {
    value.map(|v| format!("{v}")).unwrap_or_default()
}

/// `GET /api/crm/leads/export` — the branch's leads as `text/csv`, honoring
/// the same `stage`/`search` filters as the list endpoint.
pub async fn export_leads_csv(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<crate::requests::ListQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    // #1456 — bounded export; X-Total-Count reports the full branch volume
    // so the UI can show "exported N of M".
    let total: i64 = filtered_deals(branch_id, query.stage.clone(), query.search.clone())
        .count()
        .get_result(&mut conn)
        .map_err(db_err)?;
    let rows: Vec<CrmDeal> = filtered_deals(branch_id, query.stage, query.search)
        .order(crm_deals::created_at.desc())
        .limit(50_000)
        .load(&mut conn)
        .map_err(db_err)?;

    let mut out = String::from("title,first_name,last_name,email,phone,company,job_title,source,value,currency,stage\n");
    for row in &rows {
        let contact = match row.contact_id {
            Some(cid) => crm_contacts::table
                .filter(crm_contacts::id.eq(cid))
                .select((
                    crm_contacts::first_name,
                    crm_contacts::last_name,
                    crm_contacts::email,
                    crm_contacts::phone,
                    crm_contacts::job_title,
                ))
                .first::<(
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                )>(&mut conn)
                .ok(),
            None => None,
        };
        let (first, last, email, phone, job) = contact
            .map(|(f, l, e, p, j)| (f.unwrap_or_default(), l.unwrap_or_default(), e, p, j))
            .unwrap_or_default();
        let company = match row.account_id {
            Some(aid) => crm_accounts::table
                .filter(crm_accounts::id.eq(aid))
                .select(crm_accounts::name)
                .first::<String>(&mut conn)
                .unwrap_or_default(),
            None => String::new(),
        };
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_opt(&row.title),
            csv_cell(&first),
            csv_cell(&last),
            csv_opt(&email),
            csv_opt(&phone),
            csv_cell(&company),
            csv_opt(&job),
            csv_opt(&row.source),
            csv_number(row.value),
            csv_opt(&row.currency),
            csv_opt(&row.stage),
        ));
    }
    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "lead",
            entity_id: None,
            action: "export_csv",
            before: None,
            after: None,
            detail: Some(serde_json::json!({ "rows": rows.len(), "total": total })),
        },
    );
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::HeaderName::from_static("x-total-count"), total.to_string()),
        ],
        out,
    ))
}

/// Builds the branch-scoped, filter-honoring deals query. Used twice per
/// export (#1456): once for the total count, once for the bounded page.
fn filtered_deals(
    branch_id: Uuid,
    stage: Option<String>,
    search: Option<String>,
) -> crm_deals::BoxedQuery<'static, diesel::pg::Pg> {
    let mut q = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .into_boxed();
    if let Some(stage) = stage {
        q = q.filter(crm_deals::stage.eq(stage));
    }
    if let Some(search) = search {
        q = q.filter(crm_deals::title.ilike(format!("%{search}%")));
    }
    q
}

/// `POST /api/crm/leads/import` — parse a CSV body (`text/csv` or JSON rows)
/// into leads. Email is the dedupe key inside the branch; rows with no email
/// and no name are reported as errors instead of being dropped silently.
pub async fn import_leads_csv(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<ImportQuery>,
    body: String,
) -> Result<Json<CsvImportReport>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let actor = email_from_jwt(&headers);

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/csv");
    let rows: Vec<CsvLeadRow> = if content_type.contains("application/json") {
        serde_json::from_str(&body)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid JSON rows: {e}")))?
    } else {
        parse_leads_csv(&body)?
    };

    let mut report = CsvImportReport {
        imported: 0,
        skipped_duplicates: 0,
        errors: Vec::new(),
    };
    let dry_run = query.dry_run.unwrap_or(false);
    let now = chrono::Utc::now();
    for (idx, row) in rows.iter().enumerate() {
        let email = row.email.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let has_name = row.first_name.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some()
            || row.last_name.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some()
            || row.title.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some();
        if email.is_none() && !has_name {
            report.errors.push((idx + 1, "row has neither email nor name".to_string()));
            continue;
        }

        // Branch-wide dedupe on email — the same key the lead form uses.
        if let Some(mail) = email {
            let match_ids: Vec<Uuid> = crm_contacts::table
                .filter(crm_contacts::branch_id.eq(branch_id))
                .filter(crm_contacts::email.ilike(mail))
                .select(crm_contacts::id)
                .load(&mut conn)
                .unwrap_or_default();
            let exists = if match_ids.is_empty() {
                0
            } else {
                crm_deals::table
                    .filter(crm_deals::branch_id.eq(branch_id))
                    .filter(crm_deals::contact_id.eq_any(&match_ids))
                    .count()
                    .get_result::<i64>(&mut conn)
                    .unwrap_or(0)
            };
            if exists > 0 {
                report.skipped_duplicates += 1;
                continue;
            }
        }

        // #1456 — dry run: validate only, write nothing. Duplicates are
        // still detected above; everything below would create rows.
        if dry_run {
            report.imported += 1;
            continue;
        }

        let contact_id = email.and_then(|mail| {
            crm_contacts::table
                .filter(crm_contacts::branch_id.eq(branch_id))
                .filter(crm_contacts::email.ilike(mail))
                .select(crm_contacts::id)
                .first::<Uuid>(&mut conn)
                .ok()
                .or_else(|| {
                    let cid = Uuid::new_v4();
                    diesel::insert_into(crm_contacts::table)
                        .values((
                            crm_contacts::id.eq(cid),
                            crm_contacts::org_id.eq(branch_id),
                            crm_contacts::bot_id.eq(state.bot_for_branch(branch_id)),
                            crm_contacts::branch_id.eq(branch_id),
                            crm_contacts::first_name.eq(row.first_name.clone()),
                            crm_contacts::last_name.eq(row.last_name.clone()),
                            crm_contacts::email.eq(row.email.clone()),
                            crm_contacts::phone.eq(row.phone.clone()),
                            crm_contacts::company.eq(row.company.clone()),
                            crm_contacts::job_title.eq(row.job_title.clone()),
                            crm_contacts::source.eq(row.source.clone()),
                            crm_contacts::created_at.eq(now),
                            crm_contacts::updated_at.eq(now),
                        ))
                        .execute(&mut conn)
                        .ok()
                        .map(|_| cid)
                })
        });

        let title = row
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .or_else(|| {
                let name = [row.first_name.as_deref(), row.last_name.as_deref()]
                    .iter()
                    .flatten()
                    .filter(|s| !s.trim().is_empty())
                    .copied()
                    .collect::<Vec<&str>>()
                    .join(" ");
                if name.is_empty() { None } else { Some(format!("Lead — {name}")) }
            })
            .unwrap_or_else(|| "Imported lead".to_string());

        let lead = CrmDeal {
            id: Uuid::new_v4(),
            org_id: branch_id,
            bot_id: state.bot_for_branch(branch_id),
            branch_id,
            contact_id,
            account_id: None,
            am_id: None,
            lead_id: None,
            title: Some(title.clone()),
            name: Some(title),
            description: None,
            value: row.value,
            currency: row.currency.clone(),
            stage_id: None,
            stage: Some(row.stage.clone().unwrap_or_else(|| "new".to_string())),
            probability: None,
            source: row.source.clone(),
            segment_id: None,
            department_id: None,
            expected_close_date: None,
            actual_close_date: None,
            period: None,
            deal_date: None,
            won: None,
            owner_id: None,
            lost_reason: None,
            closed_at: None,
            notes: None,
            tags: None,
            custom_fields: Some(serde_json::json!({
                "imported_by": actor,
                "import_row": idx + 1,
            })),
            created_at: Some(now),
            updated_at: Some(now),
        };
        match diesel::insert_into(crm_deals::table)
            .values(&lead)
            .execute(&mut conn)
        {
            Ok(_) => {
                report.imported += 1;
                audit::record(
                    &state,
                    audit::AuditRecord {
                        headers: &headers,
                        branch_id,
                        entity: "lead",
                        entity_id: Some(lead.id),
                        action: "import_csv",
                        before: None,
                        after: Some(serde_json::to_value(&lead).unwrap_or(serde_json::Value::Null)),
                        detail: Some(serde_json::json!({ "row": idx + 1 })),
                    },
                );
            }
            Err(e) => report.errors.push((idx + 1, format!("insert failed: {e}"))),
        }
    }

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "lead",
            entity_id: None,
            action: "import_csv",
            before: None,
            after: None,
            detail: Some(serde_json::json!({
            "rows": report.imported,
            "skipped": report.skipped_duplicates,
            "errors": report.errors.len(),
            "dry_run": dry_run
        })),
        },
    );
    Ok(Json(report))
}

fn parse_leads_csv(body: &str) -> Result<Vec<CsvLeadRow>, (StatusCode, String)> {
    let mut rows = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(body.as_bytes());
    let headers = reader
        .headers()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid CSV headers: {e}")))?
        .clone();
    let idx_of = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));
    for rec in reader.records() {
        let rec = rec.map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid CSV: {e}")))?;
        let get = |name: &str| -> Option<String> {
            let idx = idx_of(name)?;
            rec.get(idx).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        };
        rows.push(CsvLeadRow {
            title: get("title"),
            first_name: get("first_name"),
            last_name: get("last_name"),
            email: get("email"),
            phone: get("phone"),
            company: get("company"),
            job_title: get("job_title"),
            source: get("source"),
            value: get("value").and_then(|v| v.replace(',', ".").parse::<f64>().ok()),
            currency: get("currency"),
            stage: get("stage"),
        });
    }
    Ok(rows)
}

// ────────────────────────────────────────────────────────────────────────────
// Audit trail reader
// ────────────────────────────────────────────────────────────────────────────

/// `GET /api/crm/audit?entity=&entity_id=&limit=&offset=` — newest-first audit
/// rows for the branch, optionally narrowed to one entity or record.
pub async fn list_audit(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<crate::models::CrmAuditLog>>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let mut q = crate::schema::crm_audit_logs::table
        .filter(crate::schema::crm_audit_logs::branch_id.eq(branch_id))
        .into_boxed();
    if let Some(entity) = query.entity {
        q = q.filter(crate::schema::crm_audit_logs::entity.eq(entity));
    }
    if let Some(entity_id) = query.entity_id {
        q = q.filter(crate::schema::crm_audit_logs::entity_id.eq(entity_id));
    }
    let rows = q
        .order(crate::schema::crm_audit_logs::created_at.desc())
        .limit(query.limit.unwrap_or(50))
        .offset(query.offset.unwrap_or(0))
        .load::<crate::models::CrmAuditLog>(&mut conn)
        .map_err(db_err)?;
    Ok(Json(rows))
}

#[derive(Debug, serde::Deserialize)]
pub struct AuditQuery {
    pub entity: Option<String>,
    pub entity_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
