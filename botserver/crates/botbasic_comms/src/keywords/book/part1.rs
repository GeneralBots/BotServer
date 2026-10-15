use super::*;

/// Selected `calendar_events` columns in select order.
type CalendarEventRow = (
    Uuid,
    String,
    Option<String>,
    DateTime<Utc>,
    DateTime<Utc>,
    Option<String>,
    String,
);

impl CalendarEngine {
    #[must_use]
    pub fn new(db: botcore::shared::utils::DbPool) -> Self {
        Self { db }
    }

    /// Persists a calendar event and returns it with the stored identifier.
    ///
    /// `calendars` and `calendar_events` carry three scope columns that are all
    /// `NOT NULL` without defaults (`org_id`, `bot_id`, `branch_id`), so the
    /// write resolves the branch scope first and provisions a default calendar
    /// when the scope has none. Callers previously received `Ok(event)` here,
    /// which made `BOOK` report a confirmed booking that was never stored.
    pub fn create_event(
        &self,
        event: CalendarEvent,
    ) -> Result<CalendarEvent, Box<dyn std::error::Error>> {
        let mut conn = self
            .db
            .get()
            .map_err(|e| format!("Database pool unavailable: {e}"))?;

        let scope = resolve_event_scope(&mut conn)?;
        let calendar_id = ensure_default_calendar(&mut conn, &scope)?;
        let owner_id = Uuid::parse_str(&event.organizer).unwrap_or(Uuid::nil());
        let reminders = match event.reminder_minutes {
            Some(minutes) => {
                serde_json::json!([{ "minutes_before": minutes, "type": "notification" }])
            }
            None => serde_json::json!([]),
        };
        let attendees = serde_json::json!(event.attendees.clone());
        let recurrence_rule = event
            .recurrence_rule
            .as_ref()
            .map(recurrence_rule_to_string);
        let status = match &event.status {
            EventStatus::Confirmed => "confirmed",
            EventStatus::Tentative => "tentative",
            EventStatus::Cancelled => "cancelled",
        };

        diesel::sql_query(
            "INSERT INTO calendar_events \
             (id, org_id, bot_id, branch_id, calendar_id, owner_id, title, description, \
              location, start_time, end_time, all_day, recurrence_rule, status, visibility, \
              busy_status, reminders, attendees, metadata) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, false, $12, $13, \
                     'default', 'busy', $14, $15, '{}')",
        )
        .bind::<diesel::sql_types::Uuid, _>(event.id)
        .bind::<diesel::sql_types::Uuid, _>(scope.org_id)
        .bind::<diesel::sql_types::Uuid, _>(scope.bot_id)
        .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
        .bind::<diesel::sql_types::Uuid, _>(calendar_id)
        .bind::<diesel::sql_types::Uuid, _>(owner_id)
        .bind::<diesel::sql_types::Text, _>(&event.title)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&event.description)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&event.location)
        .bind::<diesel::sql_types::Timestamptz, _>(event.start_time)
        .bind::<diesel::sql_types::Timestamptz, _>(event.end_time)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&recurrence_rule)
        .bind::<diesel::sql_types::Text, _>(status)
        .bind::<diesel::sql_types::Jsonb, _>(reminders)
        .bind::<diesel::sql_types::Jsonb, _>(attendees)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to persist the calendar event: {e}"))?;

        info!(
            "Persisted calendar event {} ('{}') in calendar {}",
            event.id, event.title, calendar_id
        );
        Ok(event)
    }

    pub fn check_conflicts(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        _user: &str,
    ) -> Result<Vec<CalendarEvent>, Box<dyn std::error::Error>> {
        let mut conn = self.db.get()?;

        // Find events that overlap with the given time range
        // Overlap condition: event.start < query.end AND event.end > query.start
        let rows: Vec<CalendarEventRow> = calendar_events::table
            .filter(calendar_events::start_time.lt(end))
            .filter(calendar_events::end_time.gt(start))
            .filter(calendar_events::status.ne("cancelled"))
            .select((
                calendar_events::id,
                calendar_events::title,
                calendar_events::description,
                calendar_events::start_time,
                calendar_events::end_time,
                calendar_events::location,
                calendar_events::status,
            ))
            .limit(50)
            .load(&mut conn)?;

        let events = rows.into_iter().map(|row| {
            let status = match row.6.as_str() {
                "confirmed" => EventStatus::Confirmed,
                "tentative" => EventStatus::Tentative,
                "cancelled" => EventStatus::Cancelled,
                _ => EventStatus::Confirmed,
            };

            CalendarEvent {
                id: row.0,
                title: row.1,
                description: row.2,
                start_time: row.3,
                end_time: row.4,
                location: row.5,
                organizer: String::new(),
                attendees: vec![],
                reminder_minutes: None,
                recurrence_rule: None,
                status,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }).collect();

        Ok(events)
    }

    pub fn get_events_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, Box<dyn std::error::Error>> {
        let mut conn = self.db.get()?;

        // Get all events within the time range
        let rows: Vec<CalendarEventRow> = calendar_events::table
            .filter(calendar_events::start_time.ge(start))
            .filter(calendar_events::start_time.le(end))
            .filter(calendar_events::status.ne("cancelled"))
            .order(calendar_events::start_time.asc())
            .select((
                calendar_events::id,
                calendar_events::title,
                calendar_events::description,
                calendar_events::start_time,
                calendar_events::end_time,
                calendar_events::location,
                calendar_events::status,
            ))
            .limit(100)
            .load(&mut conn)?;

        let events = rows.into_iter().map(|row| {
            let status = match row.6.as_str() {
                "confirmed" => EventStatus::Confirmed,
                "tentative" => EventStatus::Tentative,
                "cancelled" => EventStatus::Cancelled,
                _ => EventStatus::Confirmed,
            };

            CalendarEvent {
                id: row.0,
                title: row.1,
                description: row.2,
                start_time: row.3,
                end_time: row.4,
                location: row.5,
                organizer: String::new(),
                attendees: vec![],
                reminder_minutes: None,
                recurrence_rule: None,
                status,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }).collect();

        Ok(events)
    }
}
