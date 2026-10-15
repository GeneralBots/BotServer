use axum::extract::{Json, Path};
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use diesel::RunQueryDsl;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::db;
use crate::models::{CTe, NFe, NFSe, NewCTe, NewNFe, NewNFSe, Sped, TaxCalculationRequest};
use crate::storage::{ensure_schema_sync, load_rates_from_billing, parse_decimal};

/// Resolves the caller's tenant branch from the server-minted JWT claims
/// (issue #734). Falls back to the global nil branch so anonymous/system
/// callers keep working, but every query is still constrained by the resolved
/// branch — a tenant can never see another tenant's rows.
fn resolve_branch(headers: &HeaderMap) -> Uuid {
    botcore::shared::tenant::branch_from_claims(headers).unwrap_or_else(Uuid::nil)
}

pub async fn list_nfe(headers: HeaderMap) -> Result<Json<Vec<NFe>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] number: String,
        #[diesel(sql_type = diesel::sql_types::Text)] series: String,
        #[diesel(sql_type = diesel::sql_types::Text)] emitter_cnpj: String,
        #[diesel(sql_type = diesel::sql_types::Text)] recipient_cnpj: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)] total: Decimal,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)] authorized_at: Option<chrono::DateTime<Utc>>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, number, series, emitter_cnpj, recipient_cnpj, total, status, created_at, authorized_at
         FROM brazil_nfe WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| NFe {
        id: r.id, number: r.number, series: r.series, emitter_cnpj: r.emitter_cnpj,
        recipient_cnpj: r.recipient_cnpj, total: r.total.to_string(), status: r.status,
        created_at: r.created_at, authorized_at: r.authorized_at,
    }).collect()))
}

/// Fiscal documents carry Brazilian taxpayer numbers; the checker lives in
/// `botbrazil` (#1502).
fn require_cnpj(field: &str, value: &str) -> Result<(), (StatusCode, String)> {
    if botbrazil::validators::is_valid_cnpj(value) {
        Ok(())
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid {field} CNPJ '{}': check digit mismatch",
                botbrazil::validators::only_digits(value)
            ),
        ))
    }
}

pub async fn create_nfe(headers: HeaderMap, Json(req): Json<NewNFe>) -> Result<Json<NFe>, (StatusCode, String)> {
    ensure_schema_sync()?;
    require_cnpj("emitter", &req.emitter_cnpj)?;
    require_cnpj("recipient", &req.recipient_cnpj)?;
    let branch = resolve_branch(&headers);
    let total = parse_decimal(&req.total)?;
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    diesel::sql_query(
        "INSERT INTO brazil_nfe (id, number, series, emitter_cnpj, recipient_cnpj, total, status, created_at, branch_id)
         VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Text, _>(&req.number)
    .bind::<diesel::sql_types::Text, _>(&req.series)
    .bind::<diesel::sql_types::Text, _>(&req.emitter_cnpj)
    .bind::<diesel::sql_types::Text, _>(&req.recipient_cnpj)
    .bind::<diesel::sql_types::Numeric, _>(total)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(NFe {
        id, number: req.number, series: req.series, emitter_cnpj: req.emitter_cnpj,
        recipient_cnpj: req.recipient_cnpj, total: total.to_string(),
        status: "pending".to_string(), created_at: now, authorized_at: None,
    }))
}

pub async fn authorize_nfe(headers: HeaderMap, Path(id): Path<String>) -> Result<Json<NFe>, (StatusCode, String)> {
    let parsed = Uuid::parse_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid id '{id}': {e}")))?;
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let now = Utc::now();
    let n = diesel::sql_query(
        "UPDATE brazil_nfe SET status = 'authorized', authorized_at = $1 WHERE id = $2 AND branch_id = $3",
    )
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Uuid, _>(parsed)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    if n == 0 {
        return Err((StatusCode::NOT_FOUND, format!("NFe {id} not found")));
    }
    Ok(Json(NFe {
        id: parsed, number: String::new(), series: String::new(), emitter_cnpj: String::new(),
        recipient_cnpj: String::new(), total: "0".to_string(),
        status: "authorized".to_string(), created_at: now, authorized_at: Some(now),
    }))
}

pub async fn list_nfse(headers: HeaderMap) -> Result<Json<Vec<NFSe>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] number: String,
        #[diesel(sql_type = diesel::sql_types::Text)] service_code: String,
        #[diesel(sql_type = diesel::sql_types::Text)] provider_cnpj: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)] total: Decimal,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, number, service_code, provider_cnpj, total, status, created_at
         FROM brazil_nfse WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| NFSe {
        id: r.id, number: r.number, service_code: r.service_code, provider_cnpj: r.provider_cnpj,
        total: r.total.to_string(), status: r.status, created_at: r.created_at,
    }).collect()))
}

pub async fn create_nfse(headers: HeaderMap, Json(req): Json<NewNFSe>) -> Result<Json<NFSe>, (StatusCode, String)> {
    ensure_schema_sync()?;
    require_cnpj("provider", &req.provider_cnpj)?;
    let branch = resolve_branch(&headers);
    let total = parse_decimal(&req.total)?;
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    diesel::sql_query(
        "INSERT INTO brazil_nfse (id, number, service_code, provider_cnpj, total, status, created_at, branch_id)
         VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Text, _>(&req.number)
    .bind::<diesel::sql_types::Text, _>(&req.service_code)
    .bind::<diesel::sql_types::Text, _>(&req.provider_cnpj)
    .bind::<diesel::sql_types::Numeric, _>(total)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(NFSe {
        id, number: req.number, service_code: req.service_code, provider_cnpj: req.provider_cnpj,
        total: total.to_string(), status: "pending".to_string(), created_at: now,
    }))
}

pub async fn list_cte(headers: HeaderMap) -> Result<Json<Vec<CTe>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] number: String,
        #[diesel(sql_type = diesel::sql_types::Text)] sender_cnpj: String,
        #[diesel(sql_type = diesel::sql_types::Text)] recipient_cnpj: String,
        #[diesel(sql_type = diesel::sql_types::Text)] modality: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)] total: Decimal,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, number, sender_cnpj, recipient_cnpj, modality, total, status, created_at
         FROM brazil_cte WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| CTe {
        id: r.id, number: r.number, sender_cnpj: r.sender_cnpj, recipient_cnpj: r.recipient_cnpj,
        modality: r.modality, total: r.total.to_string(), status: r.status, created_at: r.created_at,
    }).collect()))
}

pub async fn create_cte(headers: HeaderMap, Json(req): Json<NewCTe>) -> Result<Json<CTe>, (StatusCode, String)> {
    ensure_schema_sync()?;
    require_cnpj("sender", &req.sender_cnpj)?;
    require_cnpj("recipient", &req.recipient_cnpj)?;
    let branch = resolve_branch(&headers);
    let total = parse_decimal(&req.total)?;
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    diesel::sql_query(
        "INSERT INTO brazil_cte (id, number, sender_cnpj, recipient_cnpj, modality, total, status, created_at, branch_id)
         VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Text, _>(&req.number)
    .bind::<diesel::sql_types::Text, _>(&req.sender_cnpj)
    .bind::<diesel::sql_types::Text, _>(&req.recipient_cnpj)
    .bind::<diesel::sql_types::Text, _>(&req.modality)
    .bind::<diesel::sql_types::Numeric, _>(total)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(CTe {
        id, number: req.number, sender_cnpj: req.sender_cnpj, recipient_cnpj: req.recipient_cnpj,
        modality: req.modality, total: total.to_string(), status: "pending".to_string(), created_at: now,
    }))
}

pub async fn list_sped(headers: HeaderMap) -> Result<Json<Vec<Sped>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] period: String,
        #[diesel(sql_type = diesel::sql_types::Text)] kind: String,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, period, kind, status, created_at FROM brazil_sped WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| Sped {
        id: r.id, period: r.period, kind: r.kind, status: r.status, created_at: r.created_at,
    }).collect()))
}

/// Calculates service taxes (issue #722). Rates come from `billing_tax_rates`
/// when present for the branch, otherwise built-in defaults. Every calculation
/// is persisted to `tax_calculations` (migration 6.5.54) for audit.
pub async fn calculate_tax(
    headers: HeaderMap,
    Json(req): Json<TaxCalculationRequest>,
) -> Result<Json<crate::calculator::TaxBreakdown>, (StatusCode, String)> {
    let service_value = parse_decimal(&req.service_value)?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;

    // Tenant scope comes exclusively from the server-minted JWT claim
    // (issue #734); the client-supplied body branch_id is never trusted, so
    // a caller cannot load another tenant's tax rates by providing its id.
    let loaded = load_rates_from_billing(&mut conn, &branch);
    let rates = loaded.unwrap_or_default();
    let breakdown = crate::calculator::calculate_service_tax(service_value, &rates);
    persist_calculation(
        &mut conn,
        &branch,
        req.service_id,
        req.service_name.as_deref(),
        &breakdown,
        if loaded.is_some() { "billing_tax_rates" } else { "default" },
    )?;
    Ok(Json(breakdown))
}

/// Inserts an audit row for a tax calculation (issue #722 acceptance criteria).
/// Shared by the REST handler and the chat `service.tax` catalog command.
pub fn persist_calculation(
    conn: &mut diesel::PgConnection,
    branch: &Uuid,
    service_id: Option<Uuid>,
    service_name: Option<&str>,
    breakdown: &crate::calculator::TaxBreakdown,
    rate_source: &str,
) -> Result<(), (StatusCode, String)> {
    let eff: Decimal = breakdown.effective_rate;
    diesel::sql_query(
        "INSERT INTO tax_calculations \
         (branch_id, service_id, service_name, service_value, irpj, csll, pis_cofins, iss, \
          total_taxes, effective_rate, rate_source) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(service_id)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(service_name)
    .bind::<diesel::sql_types::Numeric, _>(breakdown.service_value)
    .bind::<diesel::sql_types::Numeric, _>(breakdown.irpj)
    .bind::<diesel::sql_types::Numeric, _>(breakdown.csll)
    .bind::<diesel::sql_types::Numeric, _>(breakdown.pis_cofins)
    .bind::<diesel::sql_types::Numeric, _>(breakdown.iss)
    .bind::<diesel::sql_types::Numeric, _>(breakdown.total_taxes)
    .bind::<diesel::sql_types::Numeric, _>(eff)
    .bind::<diesel::sql_types::Text, _>(rate_source)
    .execute(conn)
    .map_err(db::map_diesel_err)?;
    Ok(())
}

/// Lists persisted tax calculations (issue #722), branch-scoped by the
/// server-minted JWT claim.
pub async fn list_calculations(
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)] branch_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)] service_id: Option<Uuid>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] service_name: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Numeric)] service_value: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] irpj: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] csll: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] pis_cofins: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] iss: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] total_taxes: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] effective_rate: Decimal,
        #[diesel(sql_type = diesel::sql_types::Text)] rate_source: String,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, branch_id, service_id, service_name, service_value, irpj, csll, \
         pis_cofins, iss, total_taxes, effective_rate, rate_source \
         FROM tax_calculations \
         WHERE branch_id = $1 \
         ORDER BY created_at DESC LIMIT 100",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "branch_id": r.branch_id,
                "service_id": r.service_id,
                "service_name": r.service_name,
                "service_value": r.service_value.to_string(),
                "irpj": r.irpj.to_string(),
                "csll": r.csll.to_string(),
                "pis_cofins": r.pis_cofins.to_string(),
                "iss": r.iss.to_string(),
                "total_taxes": r.total_taxes.to_string(),
                "effective_rate": r.effective_rate.to_string(),
                "rate_source": r.rate_source,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "count": items.len(), "calculations": items })))
}

#[cfg(test)]
mod tests {
    use super::require_cnpj;

    #[test]
    fn valid_cnpj_passes_the_check_digit_test() {
        assert!(require_cnpj("emitter", "11.222.333/0001-81").is_ok());
    }

    #[test]
    fn invalid_cnpj_is_rejected_with_a_400() {
        let err = require_cnpj("recipient", "11.222.333/0001-80").expect_err("bad check digit");
        assert_eq!(err.0, axum::http::StatusCode::BAD_REQUEST);
        assert!(err.1.contains("recipient"), "field name must be reported: {}", err.1);
    }
}
