//! #1451 — contacts CSV export/import (parity with the leads surface in
//! `csv_io.rs`). Email-keyed dedupe, row-level error reporting, audited.

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
use crate::models::CrmContact;
use crate::requests::CsvImportReport;
use crate::schema::crm_contacts;
use crate::scope::{branch_from_jwt, email_from_jwt};
use crate::CrateState;

fn get_bot_context(state: &CrateState) -> Uuid {
    state.get_bot_context()
}

fn db_err<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
}

fn csv_opt(value: &Option<String>) -> String {
    let raw = value.as_deref().unwrap_or("");
    if raw.contains(',') || raw.contains('"') || raw.contains('\n') {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw.to_string()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// #1451 — Contacts CSV export/import (parity with the leads surface)
// ────────────────────────────────────────────────────────────────────────────

/// `GET /api/crm/contacts/export` — the branch's contacts as `text/csv`.
pub async fn export_contacts_csv(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let rows: Vec<CrmContact> = crm_contacts::table
        .filter(crm_contacts::branch_id.eq(branch_id))
        .order(crm_contacts::created_at.desc())
        .load(&mut conn)
        .map_err(db_err)?;

    let mut out = String::from(
        "first_name,last_name,email,phone,mobile,company,job_title,status,source,city,country\n",
    );
    for r in &rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_opt(&r.first_name),
            csv_opt(&r.last_name),
            csv_opt(&r.email),
            csv_opt(&r.phone),
            csv_opt(&r.mobile),
            csv_opt(&r.company),
            csv_opt(&r.job_title),
            csv_opt(&r.status),
            csv_opt(&r.source),
            csv_opt(&r.city),
            csv_opt(&r.country),
        ));
    }
    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "contact",
            entity_id: None,
            action: "export_csv",
            before: None,
            after: None,
            detail: Some(serde_json::json!({ "rows": rows.len() })),
        },
    );
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::HeaderName::from_static("x-total-count"), rows.len().to_string()),
        ],
        out,
    ))
}

/// `POST /api/crm/contacts/import` — email-keyed dedupe; rows without email
/// AND name are reported instead of dropped. Creates contact rows only.
pub async fn import_contacts_csv(
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
    let rows: Vec<CsvContactRow> = if content_type.contains("application/json") {
        serde_json::from_str(&body)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid JSON rows: {e}")))?
    } else {
        parse_contacts_csv(&body)?
    };

    let mut report = CsvImportReport {
        imported: 0,
        skipped_duplicates: 0,
        errors: Vec::new(),
    };
    let now = chrono::Utc::now();
    let dry_run = query.dry_run.unwrap_or(false);
    for (idx, row) in rows.iter().enumerate() {
        let email = row.email.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let has_name = row.first_name.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some()
            || row.last_name.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some();
        if email.is_none() && !has_name {
            report.errors.push((idx + 1, "row has neither email nor name".to_string()));
            continue;
        }
        if let Some(mail) = email {
            let exists: i64 = crm_contacts::table
                .filter(crm_contacts::branch_id.eq(branch_id))
                .filter(crm_contacts::email.ilike(mail))
                .count()
                .get_result(&mut conn)
                .unwrap_or(0);
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

        let contact = CrmContact {
            id: Uuid::new_v4(),
            org_id: branch_id,
            bot_id: state.bot_for_branch(branch_id),
            branch_id,
            first_name: row.first_name.clone(),
            last_name: row.last_name.clone(),
            email: row.email.clone(),
            phone: row.phone.clone(),
            mobile: row.mobile.clone(),
            company: row.company.clone(),
            job_title: row.job_title.clone(),
            source: row.source.clone(),
            status: Some("lead".to_string()),
            tags: None,
            custom_fields: Some(serde_json::json!({
                "imported_by": actor,
                "import_row": idx + 1,
            })),
            notes: None,
            owner_id: None,
            pass_hash: None,
            created_at: now,
            updated_at: now,
            address_line1: None,
            address_line2: None,
            city: row.city.clone(),
            state: None,
            postal_code: None,
            country: row.country.clone(),
        };
        match diesel::insert_into(crm_contacts::table)
            .values(&contact)
            .execute(&mut conn)
        {
            Ok(_) => {
                report.imported += 1;
                audit::record(
                    &state,
                    audit::AuditRecord {
                        headers: &headers,
                        branch_id,
                        entity: "contact",
                        entity_id: Some(contact.id),
                        action: "import_csv",
                        before: None,
                        after: Some(serde_json::to_value(&contact).unwrap_or(serde_json::Value::Null)),
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
            entity: "contact",
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

/// #1456 — import options: `?dry_run=true` validates without writing.
#[derive(Debug, serde::Deserialize)]
pub struct ImportQuery {
    pub dry_run: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CsvContactRow {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
}

fn parse_contacts_csv(body: &str) -> Result<Vec<CsvContactRow>, (StatusCode, String)> {
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
        rows.push(CsvContactRow {
            first_name: get("first_name"),
            last_name: get("last_name"),
            email: get("email"),
            phone: get("phone"),
            mobile: get("mobile"),
            company: get("company"),
            job_title: get("job_title"),
            status: get("status"),
            source: get("source"),
            city: get("city"),
            country: get("country"),
        });
    }
    Ok(rows)
}
