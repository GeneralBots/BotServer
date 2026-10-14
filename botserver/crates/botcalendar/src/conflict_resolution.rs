use chrono::{DateTime, Duration, TimeZone, Utc};
use icalendar::{Component, EventLike};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcsEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub organizer: Option<String>,
    pub attendees: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub recurrence_rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionRequest {
    pub ics_data: String,
    pub calendar_id: Option<Uuid>,
    pub buffer_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionResult {
    pub parsed_events: Vec<IcsEvent>,
    pub has_conflicts: bool,
    pub conflicts: Vec<ConflictDetail>,
    pub free_slots: Vec<FreeSlot>,
    pub suggested_alternatives: Vec<String>,
    pub llm_suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetail {
    pub proposed_event: IcsEvent,
    pub conflicting_event_title: String,
    pub conflicting_event_start: DateTime<Utc>,
    pub conflicting_event_end: DateTime<Utc>,
    pub overlap_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeSlot {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_minutes: i64,
}

pub fn parse_ics(ics_data: &str) -> Result<Vec<IcsEvent>, String> {
    let calendar: icalendar::Calendar = ics_data
        .parse()
        .map_err(|e| format!("Failed to parse ICS data: {e}"))?;

    let mut events = Vec::new();

    for component in &calendar.components {
        if let icalendar::CalendarComponent::Event(event) = component {
            let uid = event
                .property_value("UID")
                .map(|u| u.to_string())
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            let summary = event
                .property_value("SUMMARY")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Untitled Event".to_string());

            let start_time = event.property_value("DTSTART")
                .and_then(parse_ical_dt);
            let end_time = event.property_value("DTEND")
                .and_then(parse_ical_dt);

            let (start, end) = match (start_time, end_time) {
                (Some(s), Some(e)) => (s, e),
                (Some(s), None) => {
                    let e = s + Duration::hours(1);
                    (s, e)
                }
                _ => continue,
            };

            let organizer = event
                .property_value("ORGANIZER")
                .map(|o| o.trim_start_matches("mailto:").to_string());

            let mut attendees = Vec::new();
            if let Some(attendee_props) = event.multi_properties().get("ATTENDEE") {
                for prop in attendee_props {
                    let val = prop.value().trim_start_matches("mailto:").to_string();
                    if !attendees.contains(&val) {
                        attendees.push(val);
                    }
                }
            }

            events.push(IcsEvent {
                uid,
                summary,
                description: event.get_description().map(String::from),
                location: event.get_location().map(String::from),
                organizer,
                attendees,
                start_time: start,
                end_time: end,
                recurrence_rule: event
                    .properties()
                    .get("RRULE")
                    .map(|p| p.value().to_string()),
            });
        }
    }

    if events.is_empty() {
        return Err("No VEVENT components found in ICS data".into());
    }

    Ok(events)
}

pub async fn resolve_conflicts(
    existing_events: &[(DateTime<Utc>, DateTime<Utc>, String)],
    proposed_events: &[IcsEvent],
    calendar_id: Option<Uuid>,
    buffer_minutes: u32,
) -> ConflictResolutionResult {
    let buffer = Duration::minutes(i64::from(buffer_minutes));
    let mut conflicts = Vec::new();
    let mut free_slots = Vec::new();
    let mut all_alternatives: Vec<String> = Vec::new();

    for proposed in proposed_events {
        let mut event_conflicts = Vec::new();

        for (existing_start, existing_end, existing_title) in existing_events {
            let adjusted_start = proposed.start_time - buffer;
            let adjusted_end = proposed.end_time + buffer;

            if adjusted_start < *existing_end && adjusted_end > *existing_start {
                let overlap_start = std::cmp::max(adjusted_start, *existing_start);
                let overlap_end = std::cmp::min(adjusted_end, *existing_end);
                let overlap_minutes = (overlap_end - overlap_start).num_minutes();

                event_conflicts.push(ConflictDetail {
                    proposed_event: proposed.clone(),
                    conflicting_event_title: existing_title.clone(),
                    conflicting_event_start: *existing_start,
                    conflicting_event_end: *existing_end,
                    overlap_minutes,
                });
            }
        }

        if event_conflicts.is_empty() {
            free_slots.push(FreeSlot {
                start: proposed.start_time,
                end: proposed.end_time,
                duration_minutes: (proposed.end_time - proposed.start_time).num_minutes(),
            });
        } else {
            let alternatives = generate_time_alternatives(proposed, 3);
            all_alternatives.extend(alternatives);
            conflicts.extend(event_conflicts);
        }
    }

    free_slots.sort_by_key(|a| a.start);

    let llm_suggestion = if conflicts.is_empty() {
        None
    } else {
        match call_llm_suggest(
            &conflicts,
            &all_alternatives,
            calendar_id,
        )
        .await
        {
            Ok(suggestion) => Some(suggestion),
            Err(e) => {
                warn!("Failed to get LLM suggestion for conflict resolution: {e}");
                None
            }
        }
    };

    let has_conflicts = !conflicts.is_empty();

    info!(
        "Conflict resolution: {} proposed events, {} conflicts detected, {} free slots",
        proposed_events.len(),
        conflicts.len(),
        free_slots.len()
    );

    ConflictResolutionResult {
        parsed_events: proposed_events.to_vec(),
        has_conflicts,
        conflicts,
        free_slots,
        suggested_alternatives: all_alternatives,
        llm_suggestion,
    }
}

fn generate_time_alternatives(event: &IcsEvent, count: usize) -> Vec<String> {
    let duration = event.end_time - event.start_time;
    let mut alternatives = Vec::new();

    let offsets = [
        Duration::hours(1),
        Duration::hours(-1),
        Duration::days(1),
        Duration::days(-1),
        Duration::hours(2),
        Duration::days(2),
    ];

    for offset in offsets.iter().take(count.saturating_sub(alternatives.len()) + 2) {
        let new_start = event.start_time + *offset;
        let new_end = new_start + duration;

        if new_start > Utc::now() {
            alternatives.push(format!(
                "{} - {}",
                new_start.to_rfc3339(),
                new_end.to_rfc3339()
            ));
        }
    }

    alternatives.truncate(count);
    alternatives
}

async fn call_llm_suggest(
    conflicts: &[ConflictDetail],
    alternatives: &[String],
    _calendar_id: Option<Uuid>,
) -> Result<String, String> {
    let llm_url = match std::env::var("BOT_EMAIL_LLM_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return Err("BOT_EMAIL_LLM_URL not set".into()),
    };

    let conflicts_text: String = conflicts
        .iter()
        .map(|c| {
            format!(
                "- Proposed: \"{}\" conflicts with existing: \"{}\" (overlap: {} min)",
                c.proposed_event.summary,
                c.conflicting_event_title,
                c.overlap_minutes
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let alternatives_text = if alternatives.is_empty() {
        "No alternatives generated yet.".to_string()
    } else {
        alternatives
            .iter()
            .enumerate()
            .map(|(i, a)| format!("{}. {a}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let prompt = format!(
        "You are a calendar conflict resolution assistant. Analyze the following conflicts and suggest \
         the best course of action.\n\nConflicts:\n{conflicts}\n\nPossible alternatives:\n{alts}\n\n\
         Provide a concise suggestion (max 3 sentences) in Portuguese on how to resolve these conflicts, \
         considering the alternatives listed.",
        conflicts = conflicts_text,
        alts = alternatives_text,
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let resp = client
        .post(&llm_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": std::env::var("BOT_EMAIL_LLM_MODEL").unwrap_or_else(|_| "default".into()),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 300,
            "temperature": 0.3,
        }))
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {e}"))?;

    resp.text()
        .await
        .map_err(|e| format!("Failed to read LLM response: {e}"))
}

fn parse_ical_dt(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    if value.ends_with('Z') {
        let without_z = value.trim_end_matches('Z');
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S") {
            return Some(Utc.from_utc_datetime(&naive));
        }
    }
    if value.len() == 8 && value.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y%m%d") {
            let naive = date.and_hms_opt(0, 0, 0)?;
            return Some(Utc.from_utc_datetime(&naive));
        }
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S") {
        return Some(Utc.from_utc_datetime(&naive));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_event(uid: &str, summary: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> IcsEvent {
        IcsEvent {
            uid: uid.into(),
            summary: summary.into(),
            description: None,
            location: None,
            organizer: None,
            attendees: vec![],
            start_time: start,
            end_time: end,
            recurrence_rule: None,
        }
    }

    #[test]
    fn test_parse_ics_empty_calendar() {
        let ics = "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Test//EN\nEND:VCALENDAR";
        let result = parse_ics(ics);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("No VEVENT components found"));
    }

    #[test]
    fn test_parse_ics_single_event() {
        let ics = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\nBEGIN:VEVENT\r\n\
                    UID:test-uid-123\r\nSUMMARY:Test Meeting\r\nDTSTART:20250101T100000Z\r\n\
                    DTEND:20250101T110000Z\r\nORGANIZER:mailto:organizer@test.com\r\n\
                    ATTENDEE:mailto:attendee1@test.com\r\nATTENDEE:mailto:attendee2@test.com\r\n\
                    END:VEVENT\r\nEND:VCALENDAR";
        let events = parse_ics(ics).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Test Meeting");
        assert_eq!(events[0].uid, "test-uid-123");
        assert_eq!(events[0].attendees.len(), 2);
    }

    #[test]
    fn test_no_conflicts() {
        let now = Utc::now();
        let existing = vec![
            (now + Duration::hours(2), now + Duration::hours(3), "Existing".into()),
        ];
        let proposed = vec![test_event("p1", "Proposed", now + Duration::hours(5), now + Duration::hours(6))];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_conflicts(&existing, &proposed, None, 15));
        assert!(!result.has_conflicts);
        assert_eq!(result.free_slots.len(), 1);
        assert!(result.llm_suggestion.is_none());
    }

    #[test]
    fn test_direct_conflict() {
        let now = Utc::now();
        let existing = vec![
            (now + Duration::hours(2), now + Duration::hours(4), "Existing".into()),
        ];
        let proposed = vec![test_event("p1", "Conflicting", now + Duration::hours(3), now + Duration::hours(5))];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_conflicts(&existing, &proposed, None, 0));
        assert!(result.has_conflicts);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(
            result.conflicts[0].conflicting_event_title,
            "Existing"
        );
        assert!(result.conflicts[0].overlap_minutes > 0);
    }

    #[test]
    fn test_free_slots_sorted() {
        let now = Utc::now();
        let existing = vec![];
        let proposed = vec![
            test_event("p1", "Slot 1", now + Duration::hours(3), now + Duration::hours(4)),
            test_event("p2", "Slot 2", now + Duration::hours(1), now + Duration::hours(2)),
        ];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_conflicts(&existing, &proposed, None, 0));
        assert!(!result.has_conflicts);
        assert_eq!(result.free_slots.len(), 2);
        assert!(result.free_slots[0].start < result.free_slots[1].start);
    }

    #[test]
    fn test_generate_time_alternatives_returns_future_times() {
        let now = Utc::now();
        let event = IcsEvent {
            uid: "test".into(),
            summary: "Test".into(),
            description: None,
            location: None,
            organizer: None,
            attendees: vec![],
            start_time: now - Duration::hours(48),
            end_time: now - Duration::hours(47),
            recurrence_rule: None,
        };
        let alts = generate_time_alternatives(&event, 3);
        assert!(alts.len() <= 3);
    }
}
