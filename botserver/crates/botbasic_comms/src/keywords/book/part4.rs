use super::*;

/// Resolves the scope used for calendar writes from the oldest branch, using
/// that branch's default bot. Mirrors the resolution performed by the server
/// bootstrap so bookings land in the same scope the Calendar application reads.
pub(super) fn resolve_event_scope(
    conn: &mut diesel::PgConnection,
) -> Result<EventScope, Box<dyn std::error::Error>> {
    #[derive(diesel::QueryableByName)]
    struct ScopeRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        branch_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        org_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        bot_id: Uuid,
    }

    let row = diesel::sql_query(
        "SELECT b.id AS branch_id, b.org_id, \
                COALESCE((SELECT id FROM bots WHERE branch_id = b.id \
                          ORDER BY is_default_for_branch DESC, created_at ASC LIMIT 1), \
                         '00000000-0000-0000-0000-000000000000') AS bot_id \
         FROM branches b ORDER BY b.created_at ASC LIMIT 1",
    )
    .get_result::<ScopeRow>(conn)
    .optional()?;

    match row {
        Some(r) => Ok(EventScope {
            org_id: r.org_id,
            bot_id: r.bot_id,
            branch_id: r.branch_id,
        }),
        None => Err("No branch is configured, so the event cannot be scoped".into()),
    }
}
/// Returns the identifier of the scope's primary calendar, provisioning one
/// when the scope has none. `calendar_events.calendar_id` is `NOT NULL`, so a
/// booking cannot be stored until a calendar exists.
pub(super) fn ensure_default_calendar(
    conn: &mut diesel::PgConnection,
    scope: &EventScope,
) -> Result<Uuid, Box<dyn std::error::Error>> {
    #[derive(diesel::QueryableByName)]
    struct CalendarIdRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }

    let existing = diesel::sql_query(
        "SELECT id FROM calendars WHERE org_id = $1 AND bot_id = $2 AND branch_id = $3 \
         ORDER BY is_primary DESC, created_at ASC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(scope.org_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.bot_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
    .get_result::<CalendarIdRow>(conn)
    .optional()?;

    if let Some(row) = existing {
        return Ok(row.id);
    }

    let calendar_id = Uuid::new_v4();
    diesel::sql_query(
        "INSERT INTO calendars \
         (id, org_id, bot_id, branch_id, owner_id, name, description, color, timezone, \
          is_primary, is_visible, is_shared) \
         VALUES ($1, $2, $3, $4, '00000000-0000-0000-0000-000000000000', 'Default', \
                 'Your default calendar', '#3b82f6', 'UTC', true, true, false)",
    )
    .bind::<diesel::sql_types::Uuid, _>(calendar_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.org_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.bot_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
    .execute(conn)?;

    info!(
        "Provisioned default calendar {calendar_id} for branch {}",
        scope.branch_id
    );
    Ok(calendar_id)
}
/// Renders the in-memory recurrence rule as an iCalendar `RRULE` value, which
/// is the representation stored in `calendar_events.recurrence_rule`.
pub(super) fn recurrence_rule_to_string(rule: &RecurrenceRule) -> String {
    let mut parts = vec![format!("FREQ={}", rule.frequency.to_uppercase())];
    if rule.interval > 0 {
        parts.push(format!("INTERVAL={}", rule.interval));
    }
    if let Some(count) = rule.count {
        parts.push(format!("COUNT={count}"));
    }
    if let Some(until) = rule.until {
        parts.push(format!("UNTIL={}", until.format("%Y%m%dT%H%M%SZ")));
    }
    if let Some(days) = rule.by_day.as_ref().filter(|days| !days.is_empty()) {
        parts.push(format!("BYDAY={}", days.join(",")));
    }
    parts.join(";")
}
pub(super) fn send_meeting_invite(
    _state: &dyn BasicRuntime,
    event: &CalendarEvent,
    attendee: &str,
) -> Result<(), String> {
    info!(
        "Would send meeting invite for '{}' to {}",
        event.title, attendee
    );
    Ok(())
}
