//! Calendar queries and calendar-object documents for the CalDAV surface
//! (issue #1335).
//!
//! Every query is constrained to the caller's scope (the `org_id`/`bot_id` pair
//! resolved from the authenticated request), so a client can never reach a
//! calendar of another tenant by guessing an identifier.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use botsecurity_core::response_error::ResponseError;

use super::{calendar_events, calendars, record_to_event, CalendarEventRecord, CalendarRecord, DbPool};
use crate::caldav_http::{not_found, unavailable};
use crate::caldav_xml;

/// Upper bound on the events one `REPORT` may return, so a client that asks for
/// an unbounded range cannot make the server materialize an entire table.
pub(crate) const REPORT_EVENT_LIMIT: i64 = 2000;

/// Renders a failed store operation as the response the request must return.
///
/// The boxed wrapper keeps the `Result` of every loader small
/// (`clippy::result_large_err`); callers render it with `IntoResponse`.
fn store_error(message: String) -> ResponseError {
    unavailable(message).into()
}

/// Resolves the calendars visible to the caller's scope.
pub(crate) async fn load_calendars(
    pool: &Arc<DbPool>,
    headers: &axum::http::HeaderMap,
) -> Result<Vec<CalendarRecord>, ResponseError> {
    let (org_id, bot_id) = super::scope_from_headers(pool, headers);
    let pool = pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
        calendars::table
            .filter(calendars::org_id.eq(org_id))
            .filter(calendars::bot_id.eq(bot_id))
            .order(calendars::created_at.asc())
            .load::<CalendarRecord>(&mut conn)
            .map_err(|e| format!("Failed to load calendars: {e}"))
    })
    .await
    .map_err(|e| store_error(format!("Task join error: {e}")))?;
    result.map_err(store_error)
}

/// Resolves one calendar, refusing a calendar outside the caller's scope.
pub(crate) async fn load_calendar(
    pool: &Arc<DbPool>,
    headers: &axum::http::HeaderMap,
    calendar_id: Uuid,
) -> Result<Option<CalendarRecord>, ResponseError> {
    let (org_id, bot_id) = super::scope_from_headers(pool, headers);
    let pool = pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
        calendars::table
            .filter(calendars::org_id.eq(org_id))
            .filter(calendars::bot_id.eq(bot_id))
            .find(calendar_id)
            .first::<CalendarRecord>(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load the calendar: {e}"))
    })
    .await
    .map_err(|e| store_error(format!("Task join error: {e}")))?;
    result.map_err(store_error)
}

/// Resolves the events of a calendar.
///
/// The range is applied as an overlap test (`end > range start` and
/// `start < range end`), so an event that begins before the range and overlaps
/// it is returned, which is what a client expects. When the request names
/// events, only those are read.
pub(crate) async fn load_events(
    pool: &Arc<DbPool>,
    headers: &axum::http::HeaderMap,
    calendar_id: Uuid,
    time_min: Option<DateTime<Utc>>,
    time_max: Option<DateTime<Utc>>,
    wanted: Vec<Uuid>,
) -> Result<Vec<CalendarEventRecord>, ResponseError> {
    let (org_id, bot_id) = super::scope_from_headers(pool, headers);
    let pool = pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
        let mut query = calendar_events::table
            .filter(calendar_events::org_id.eq(org_id))
            .filter(calendar_events::bot_id.eq(bot_id))
            .filter(calendar_events::calendar_id.eq(calendar_id))
            .into_boxed();
        if let Some(time_min) = time_min {
            query = query.filter(calendar_events::end_time.gt(time_min));
        }
        if let Some(time_max) = time_max {
            query = query.filter(calendar_events::start_time.lt(time_max));
        }
        if !wanted.is_empty() {
            query = query.filter(calendar_events::id.eq_any(wanted));
        }
        query
            .order(calendar_events::start_time.asc())
            .limit(REPORT_EVENT_LIMIT)
            .load::<CalendarEventRecord>(&mut conn)
            .map_err(|e| format!("Failed to load events: {e}"))
    })
    .await
    .map_err(|e| store_error(format!("Task join error: {e}")))?;
    result.map_err(store_error)
}

/// Removes an event from the caller's scope.
pub(crate) async fn delete_event(
    pool: &Arc<DbPool>,
    headers: &axum::http::HeaderMap,
    event_id: Uuid,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let (org_id, bot_id) = super::scope_from_headers(pool, headers);
    let pool = pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
        diesel::delete(
            calendar_events::table
                .filter(calendar_events::org_id.eq(org_id))
                .filter(calendar_events::bot_id.eq(bot_id))
                .filter(calendar_events::id.eq(event_id)),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete the event: {e}"))
    })
    .await;

    match result {
        Ok(Ok(0)) => not_found("Unknown event"),
        Ok(Ok(_)) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Ok(Err(message)) => delete_failed(&message),
        Err(e) => delete_failed(&e.to_string()),
    }
}

fn delete_failed(reason: &str) -> axum::response::Response {
    use axum::response::IntoResponse;

    log::error!("CalDAV delete failed: {reason}");
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "The event could not be deleted.",
    )
        .into_response()
}

/// Properties of a calendar collection, including the component set the client
/// uses to decide what it may store and a change tag for cheap polling.
pub(crate) fn calendar_response(calendar: &CalendarRecord) -> String {
    let props = format!(
        "<D:resourcetype><D:collection/><C:calendar/></D:resourcetype>\n\
         <D:displayname>{name}</D:displayname>\n\
         <C:supported-calendar-component-set><C:comp name=\"VEVENT\"/><C:comp name=\"VTODO\"/></C:supported-calendar-component-set>\n\
         <CS:getctag xmlns:CS=\"http://calendarserver.org/ns/\">{ctag}</CS:getctag>",
        name = caldav_xml::xml_escape(&calendar.name),
        ctag = calendar.updated_at.timestamp(),
    );
    caldav_xml::response(&format!("/caldav/calendars/{}/", calendar.id), &props)
}

/// Properties of a calendar object resource. `calendar-data` carries the
/// serialized event and is only sent when the client asked for the body.
pub(crate) fn event_response(
    calendar_id: Uuid,
    record: &CalendarEventRecord,
    include_data: bool,
) -> String {
    let href = format!("/caldav/calendars/{calendar_id}/{}.ics", record.id);
    let mut props = format!(
        "<D:getetag>\"{}\"</D:getetag>\n\
         <D:getcontenttype>text/calendar; charset=utf-8</D:getcontenttype>",
        record.updated_at.timestamp(),
    );
    if include_data {
        let ical = super::export_to_ical(&[record_to_event(record.clone())], "Calendar");
        props.push_str(&format!(
            "\n<C:calendar-data>{}</C:calendar-data>",
            caldav_xml::xml_escape(&ical)
        ));
    }
    caldav_xml::response(&href, &props)
}
