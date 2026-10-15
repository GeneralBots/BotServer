use axum::extract::{Json, Path};
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use diesel::RunQueryDsl;

use crate::db;
use crate::storage::ensure_schema_sync;

use botcore::shared::tenant::branch_from_claims;

fn resolve_branch(headers: &HeaderMap) -> Uuid {
    branch_from_claims(headers).unwrap_or_else(Uuid::nil)
}

/// CLT band split of a requested overtime, owned by `botclock` (#1502).
struct OvertimeBands {
    first_rate_min: u32,
    rest_rate_min: u32,
    first_rate_bps: u32,
    rest_rate_bps: u32,
}

/// Converts a requested overtime in hours to whole minutes and splits it into
/// the first band (50%, up to two hours) and the rest band (100%). Returns
/// `None` when the value is negative or exceeds whole-minute precision, so the
/// caller answers 400 instead of silently recording zero minutes.
fn overtime_bands(hours: rust_decimal::Decimal) -> Option<OvertimeBands> {
    let rules = botclock::rules::WorkdayRules::brazil_clt();
    let minutes = (hours * rust_decimal::Decimal::from(60))
        .round_dp(0)
        .to_u32()?;
    let summary = botclock::payroll::compute_summary(
        String::new(),
        Uuid::nil(),
        rules.max_daily_min.saturating_add(minutes),
        &rules,
    );
    Some(OvertimeBands {
        first_rate_min: summary.overtime.first_rate_min,
        rest_rate_min: summary.overtime.rest_rate_min,
        first_rate_bps: rules.overtime_first_rate_bps,
        rest_rate_bps: rules.overtime_rest_rate_bps,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockEvent {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub kind: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRecord {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub date: chrono::NaiveDate,
    pub clock_in: chrono::DateTime<Utc>,
    pub clock_out: Option<chrono::DateTime<Utc>>,
    pub hours_worked: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvertimeRequest {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub date: chrono::NaiveDate,
    pub hours: String,
    pub reason: String,
    pub status: String,
    pub approved_by: Option<Uuid>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: Uuid,
    pub period: String,
    pub total_hours: String,
    pub overtime_hours: String,
    pub employees: i64,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NewClockEvent {
    pub employee_id: Uuid,
    pub kind: String,
    pub notes: Option<String>,
}

pub async fn clock_in_out(headers: HeaderMap, Json(req): Json<NewClockEvent>) -> Result<Json<ClockEvent>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    diesel::sql_query(
        "INSERT INTO timeclock_events (id, employee_id, kind, ts, notes, branch_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(req.employee_id)
    .bind::<diesel::sql_types::Text, _>(&req.kind)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(req.notes.as_deref())
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(ClockEvent {
        id, employee_id: req.employee_id, kind: req.kind, timestamp: now, notes: req.notes,
    }))
}

pub async fn list_records(headers: HeaderMap) -> Result<Json<Vec<TimeRecord>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)] employee_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Date)] date: chrono::NaiveDate,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] clock_in: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)] clock_out: Option<chrono::DateTime<Utc>>,
        #[diesel(sql_type = diesel::sql_types::Numeric)] hours_worked: rust_decimal::Decimal,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, employee_id, date, clock_in, clock_out, hours_worked, status
         FROM timeclock_records WHERE branch_id = $1 ORDER BY date DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| TimeRecord {
        id: r.id, employee_id: r.employee_id, date: r.date, clock_in: r.clock_in,
        clock_out: r.clock_out, hours_worked: r.hours_worked.to_string(), status: r.status,
    }).collect()))
}

pub async fn list_overtime(headers: HeaderMap) -> Result<Json<Vec<OvertimeRequest>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)] employee_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Date)] date: chrono::NaiveDate,
        #[diesel(sql_type = diesel::sql_types::Numeric)] hours: rust_decimal::Decimal,
        #[diesel(sql_type = diesel::sql_types::Text)] reason: String,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)] approved_by: Option<Uuid>,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, employee_id, date, hours, reason, status, approved_by, created_at
         FROM timeclock_overtime WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| OvertimeRequest {
        id: r.id, employee_id: r.employee_id, date: r.date, hours: r.hours.to_string(),
        reason: r.reason, status: r.status, approved_by: r.approved_by, created_at: r.created_at,
    }).collect()))
}

pub async fn approve_overtime(headers: HeaderMap, Path(id): Path<String>) -> Result<Json<OvertimeRequest>, (StatusCode, String)> {
    let parsed = Uuid::parse_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid id '{id}': {e}")))?;
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let n = diesel::sql_query(
        "UPDATE timeclock_overtime SET status = 'approved', approved_by = gen_random_uuid() WHERE id = $1 AND branch_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(parsed)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    if n == 0 {
        return Err((StatusCode::NOT_FOUND, format!("Overtime {id} not found")));
    }
    Ok(Json(OvertimeRequest {
        id: parsed, employee_id: Uuid::nil(), date: Utc::now().date_naive(), hours: "0".to_string(),
        reason: String::new(), status: "approved".to_string(), approved_by: Some(Uuid::new_v4()),
        created_at: Utc::now(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct NewOvertimeRequest {
    pub employee_id: Uuid,
    pub date: String,
    pub hours: String,
    pub reason: String,
}

pub async fn create_overtime(
    headers: HeaderMap,
    Json(req): Json<NewOvertimeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let date = chrono::NaiveDate::parse_from_str(&req.date, "%Y-%m-%d")
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid date: {e}")))?;
    let hours = req
        .hours
        .parse::<rust_decimal::Decimal>()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid hours: {e}")))?;
    let id = Uuid::new_v4();
    // CLT policy (#1502): the overtime splits into the first band (50%, up to
    // 2h) and the rest band (100%); `botclock` owns the thresholds.
    let bands = overtime_bands(hours).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid hours '{}': must be a non-negative number of hours",
                req.hours
            ),
        )
    })?;
    diesel::sql_query(
        "INSERT INTO timeclock_overtime (id, employee_id, date, hours, reason, status, branch_id)
         VALUES ($1, $2, $3, $4, $5, 'pending', $6)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(req.employee_id)
    .bind::<diesel::sql_types::Date, _>(date)
    .bind::<diesel::sql_types::Numeric, _>(hours)
    .bind::<diesel::sql_types::Text, _>(&req.reason)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
        "overtime_first_rate_min": bands.first_rate_min,
        "overtime_rest_rate_min": bands.rest_rate_min,
        "overtime_first_rate_bps": bands.first_rate_bps,
        "overtime_rest_rate_bps": bands.rest_rate_bps,
    })))
}

pub async fn get_reports(headers: HeaderMap) -> Result<Json<Vec<Report>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] period: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)] total_hours: rust_decimal::Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] overtime_hours: rust_decimal::Decimal,
        #[diesel(sql_type = diesel::sql_types::BigInt)] employees: i64,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, period, total_hours, overtime_hours, employees, created_at
         FROM timeclock_reports WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| Report {
        id: r.id, period: r.period, total_hours: r.total_hours.to_string(),
        overtime_hours: r.overtime_hours.to_string(), employees: r.employees, created_at: r.created_at,
    }).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn bands(hours: &str) -> Option<OvertimeBands> {
        match hours.parse::<Decimal>() {
            Ok(value) => overtime_bands(value),
            Err(_) => None,
        }
    }

    #[test]
    fn overtime_within_first_band_uses_fifty_percent_only() {
        let split = bands("1.5").expect("1.5h overtime is valid");
        assert_eq!(split.first_rate_min, 90);
        assert_eq!(split.rest_rate_min, 0);
    }

    #[test]
    fn overtime_beyond_two_hours_splits_into_both_bands() {
        let split = bands("3").expect("3h overtime is valid");
        assert_eq!(split.first_rate_min, 120);
        assert_eq!(split.rest_rate_min, 60);
        assert!(split.rest_rate_bps > split.first_rate_bps);
    }

    #[test]
    fn negative_hours_are_rejected_and_fractions_round_to_minutes() {
        assert!(bands("-1").is_none());
        let half_hour = bands("0.5").expect("0.5h overtime is valid");
        assert_eq!(half_hour.first_rate_min, 30);
    }
}
