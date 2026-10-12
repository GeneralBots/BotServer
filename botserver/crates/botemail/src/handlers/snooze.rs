use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Datelike, Duration, Utc};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::AppState;
use crate::types::{SnoozeRequest, SnoozeResponse};

pub async fn snooze_emails(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SnoozeRequest>,
) -> Result<Json<SnoozeResponse>, StatusCode> {
    let mut conn = state.pool.get().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let snooze_until = calculate_snooze_time(&req.preset);
    let snooze_until_naive = snooze_until.naive_utc();

    let mut snoozed_count = 0;
    for email_id in &req.email_ids {
        diesel::sql_query(
            "INSERT INTO email_snooze (email_id, snooze_until) VALUES ($1, $2)"
        )
        .bind::<diesel::sql_types::Uuid, _>(email_id)
        .bind::<diesel::sql_types::Timestamp, _>(snooze_until_naive)
        .execute(&mut conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        snoozed_count += 1;
    }

    Ok(Json(SnoozeResponse {
        snoozed_count,
        snooze_until,
    }))
}

pub async fn get_snoozed_emails(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Uuid>>, StatusCode> {
    let mut conn = state.pool.get().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = Utc::now().naive_utc();

    #[derive(QueryableByName)]
    struct SnoozeRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        email_id: Uuid,
    }

    let rows: Vec<SnoozeRow> = diesel::sql_query(
        "SELECT email_id FROM email_snooze WHERE snooze_until <= $1"
    )
    .bind::<diesel::sql_types::Timestamp, _>(now)
    .load(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let email_ids: Vec<Uuid> = rows.into_iter().map(|r| r.email_id).collect();

    diesel::sql_query("DELETE FROM email_snooze WHERE snooze_until <= $1")
        .bind::<diesel::sql_types::Timestamp, _>(now)
        .execute(&mut conn)
        .ok();

    Ok(Json(email_ids))
}

/// Resolves a snooze preset into an instant that is always in the future.
///
/// A preset names a wall-clock slot ("later today at 18:00") which is already
/// gone once the day has moved past it; without this guard the message is
/// snoozed into the past and resurfaces immediately. When the slot has passed,
/// `rollback` days are added to reach the next occurrence.
fn calculate_snooze_time(preset: &str) -> DateTime<Utc> {
    let now = Utc::now();

    let slot = |hour: u32, day_offset: i64, rollback: Duration| {
        let day = (now + Duration::days(day_offset)).date_naive();
        let candidate = day
            .and_hms_opt(hour, 0, 0)
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or(now + Duration::days(day_offset));
        if candidate > now {
            candidate
        } else {
            candidate + rollback
        }
    };

    match preset {
        "later-today" => slot(18, 0, Duration::days(1)),
        "tomorrow" => slot(8, 1, Duration::days(1)),
        "this-weekend" => {
            let days_until_saturday = (6 - now.weekday().num_days_from_monday()) % 7;
            slot(9, days_until_saturday as i64, Duration::days(7))
        }
        "next-week" => {
            let days_until_next_monday = (7 - now.weekday().num_days_from_monday() + 1) % 7 + 7;
            slot(8, days_until_next_monday as i64, Duration::days(7))
        }
        _ => now + Duration::hours(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_snooze_time() {
        // Every preset must resolve to an instant strictly in the future, even
        // once the slot it names has already passed.
        for preset in [
            "later-today",
            "tomorrow",
            "this-weekend",
            "next-week",
            "unknown",
        ] {
            let resolved = calculate_snooze_time(preset);
            assert!(
                resolved > Utc::now(),
                "preset {preset} resolved to the past: {resolved}"
            );
        }
    }
}
