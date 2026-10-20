use super::*;

pub(super) fn execute_book_meeting(
    state: &dyn BasicRuntime,
    user: &UserSession,
    meeting_json: String,
    attendees: Vec<String>,
) -> Result<String, String> {
    let meeting_data: serde_json::Value = serde_json::from_str(&meeting_json)
        .map_err(|e| format!("Invalid meeting details: {}", e))?;

    let title = meeting_data["title"]
        .as_str()
        .ok_or("Missing meeting title")?;
    let start_time_str = meeting_data["start_time"]
        .as_str()
        .ok_or("Missing start time")?;
    let duration_minutes = meeting_data["duration"].as_i64().unwrap_or(60);
    let description = meeting_data["description"].as_str().unwrap_or("");
    let location = meeting_data["location"].as_str().unwrap_or("");
    let recurring = meeting_data["recurring"].as_bool().unwrap_or(false);

    let start_time = parse_time_string(start_time_str)?;
    let end_time = start_time + Duration::minutes(duration_minutes);

    let calendar_engine = get_calendar_engine(state)?;

    for attendee in &attendees {
        let conflicts = calendar_engine
            .check_conflicts(start_time, end_time, attendee)
            .map_err(|e| format!("Failed to check conflicts: {}", e))?;

        if !conflicts.is_empty() {
            return Err(format!("Attendee {} has a conflict at this time", attendee));
        }
    }

    let recurrence_rule = if recurring {
        Some(RecurrenceRule {
            frequency: "WEEKLY".to_string(),
            interval: 1,
            count: Some(10),
            until: None,
            by_day: None,
        })
    } else {
        None
    };

    let event = CalendarEvent {
        id: Uuid::new_v4(),
        title: title.to_string(),
        description: Some(description.to_string()),
        start_time,
        end_time,
        location: if location.is_empty() {
            None
        } else {
            Some(location.to_string())
        },
        organizer: user.user_id.to_string(),
        attendees: attendees.clone(),
        reminder_minutes: Some(30),
        recurrence_rule,
        status: EventStatus::Confirmed,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_event = calendar_engine
        .create_event(event)
        .map_err(|e| format!("Failed to create meeting: {}", e))?;

    for attendee in &attendees {
        send_meeting_invite(state, &created_event, attendee)?;
    }

    info!(
        "Meeting booked: {} at {} with {} attendees",
        title,
        start_time,
        attendees.len()
    );

    Ok(format!(
        "Meeting '{}' scheduled for {} with {} attendees (ID: {})",
        title,
        start_time.format("%Y-%m-%d %H:%M"),
        attendees.len(),
        created_event.id
    ))
}
pub(super) fn check_availability(
    state: &dyn BasicRuntime,
    date_str: &str,
    duration_minutes: i64,
) -> Result<String, String> {
    let date = parse_date_string(date_str)?;
    let calendar_engine = get_calendar_engine(state)?;

    let build_at = |hour: u32| -> Result<DateTime<Utc>, String> {
        date.date_naive()
            .and_hms_opt(hour, 0, 0)
            .map(|naive| DateTime::from_naive_utc_and_offset(naive, Utc))
            .ok_or_else(|| format!("Could not build the business-hours window for {date}"))
    };
    let business_start = build_at(9)?;
    let business_end = build_at(17)?;

    let events = calendar_engine
        .get_events_range(business_start, business_end)
        .map_err(|e| format!("Failed to get events: {}", e))?;

    let mut available_slots = Vec::new();
    let mut current_time = business_start;
    let slot_duration = Duration::minutes(duration_minutes);

    for event in &events {
        if current_time + slot_duration <= event.start_time {
            available_slots.push(format!(
                "{} - {}",
                current_time.format("%H:%M"),
                (current_time + slot_duration).format("%H:%M")
            ));
        }
        current_time = event.end_time;
    }

    if current_time + slot_duration <= business_end {
        available_slots.push(format!(
            "{} - {}",
            current_time.format("%H:%M"),
            (current_time + slot_duration).format("%H:%M")
        ));
    }

    if available_slots.is_empty() {
        Ok("No available slots on this date".to_string())
    } else {
        Ok(format!(
            "Available slots on {}: {}",
            date.format("%Y-%m-%d"),
            available_slots.join(", ")
        ))
    }
}
pub(super) fn parse_time_string(time_str: &str) -> Result<DateTime<Utc>, String> {
    let formats = vec![
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
        "%d/%m/%Y %H:%M",
        "%Y-%m-%dT%H:%M:%S",
    ];

    for format in formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(time_str, format) {
            return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
        }
    }

    if time_str.contains("tomorrow") {
        let tomorrow = Utc::now() + Duration::days(1);
        if let Some(hour) = extract_hour_from_string(time_str) {
            let at_hour = tomorrow
                .date_naive()
                .and_hms_opt(hour.min(23), 0, 0)
                .map(|naive| DateTime::from_naive_utc_and_offset(naive, Utc))
                .ok_or_else(|| format!("Could not parse time: {time_str}"))?;
            return Ok(at_hour);
        }
    }

    if time_str.starts_with("in ") {
        if let Ok(hours) = time_str
            .trim_start_matches("in ")
            .trim_end_matches(" hours")
            .trim_end_matches(" hour")
            .parse::<i64>()
        {
            return Ok(Utc::now() + Duration::hours(hours));
        }
    }

    Err(format!("Could not parse time: {}", time_str))
}
pub(super) fn parse_date_string(date_str: &str) -> Result<DateTime<Utc>, String> {
    if date_str == "today" {
        return Ok(Utc::now());
    } else if date_str == "tomorrow" {
        return Ok(Utc::now() + Duration::days(1));
    }

    let formats = vec!["%Y-%m-%d", "%Y/%m/%d", "%d/%m/%Y"];

    for format in formats {
        if let Ok(dt) = chrono::NaiveDate::parse_from_str(date_str, format) {
            if let Some(naive) = dt.and_hms_opt(0, 0, 0) {
                return Ok(naive.and_utc());
            }
        }
    }

    Err(format!("Could not parse date: {}", date_str))
}
pub(super) fn extract_hour_from_string(s: &str) -> Option<u32> {
    let s = s.to_lowercase();

    if s.contains("pm") {
        if let Some(hour_str) = s.split('p').next() {
            if let Ok(hour) = hour_str.trim().replace(':', "").parse::<u32>() {
                return Some(if hour < 12 { hour + 12 } else { hour });
            }
        }
    } else if s.contains("am") {
        if let Some(hour_str) = s.split('a').next() {
            if let Ok(hour) = hour_str.trim().replace(':', "").parse::<u32>() {
                return Some(if hour == 12 { 0 } else { hour });
            }
        }
    }

    None
}
pub(super) fn log_booking(
    state: &dyn BasicRuntime,
    user: &UserSession,
    event_id: &str,
    title: &str,
) -> Result<(), String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;
    let user_uuid = Uuid::parse_str(&user.user_id)
        .map_err(|e| format!("Invalid user id: {}", e))?;

    diesel::sql_query(
        "INSERT INTO booking_logs (id, user_id, bot_id, event_id, event_title, booked_at)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_uuid)
    .bind::<diesel::sql_types::Uuid, _>(&user.bot_id)
    .bind::<diesel::sql_types::Text, _>(event_id)
    .bind::<diesel::sql_types::Text, _>(title)
    .execute(&mut *conn)
    .map_err(|e| format!("Failed to log booking: {}", e))?;

    Ok(())
}
pub(super) fn get_calendar_engine(state: &dyn BasicRuntime) -> Result<Arc<CalendarEngine>, String> {
    let calendar_engine = Arc::new(CalendarEngine::new(state.db_pool().clone()));
    Ok(calendar_engine)
}
