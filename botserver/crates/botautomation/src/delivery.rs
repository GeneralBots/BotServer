//! Notification delivery for finished automation runs: e-mail to the
//! schedule owner plus optional SMS/channel fan-out through the injected
//! `DeliveryFn`, with bounded retries and persisted delivery status.

use crate::models::{AgentRun, AgentSchedule, DeliveryPrefs};
use crate::state::AutomationService;
use diesel::prelude::*;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Sleep before each of the three retries: none, 1 s, 2 s, 4 s.
const RETRY_DELAYS_MS: [u64; 4] = [0, 1000, 2000, 4000];

/// Renders the notification subject and body for a finished run.
pub fn format_notification(schedule: &AgentSchedule, run: &AgentRun) -> (String, String) {
    let subject = format!("[Automation] {} — {}", schedule.title, run.status);
    let mut body = format!(
        "Automation run completed.\n\nTitle: {}\nGoal: {}\nRun: {}\nTrigger: {}\nStatus: {}\n",
        schedule.title, schedule.goal, run.id, run.trigger_kind, run.status
    );
    if let Some(summary) = run.result_summary.as_deref().filter(|s| !s.is_empty()) {
        body.push_str(&format!("\nSummary:\n{summary}\n"));
    }
    if let Some(error) = run.error.as_deref().filter(|s| !s.is_empty()) {
        body.push_str(&format!("\nError:\n{error}\n"));
    }
    if let Some(finished_at) = run.finished_at {
        body.push_str(&format!("\nFinished at: {}\n", finished_at.to_rfc3339()));
    }
    (subject, body)
}

/// Sends once per entry in `RETRY_DELAYS_MS`; returns the last error when
/// every attempt fails.
fn attempt_with_retries(
    deliver: &crate::state::DeliveryFn,
    channel: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let mut last_error = String::new();
    for (index, delay_ms) in RETRY_DELAYS_MS.iter().enumerate() {
        if *delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(*delay_ms));
        }
        match deliver(channel, to, subject, body) {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::error!(
                    "automation delivery attempt {}/{} failed on {channel} to {}: {e}",
                    index + 1,
                    RETRY_DELAYS_MS.len(),
                    mask_destination(to)
                );
                last_error = e;
            }
        }
    }
    Err(last_error)
}

/// Delivers the run notification according to the schedule's preferences and
/// returns the JSON document persisted into `agent_runs.delivery_status`.
/// A `None` schedule (deleted before completion) yields a `skipped` status.
pub async fn dispatch(
    state: Arc<AutomationService>,
    schedule: Option<&AgentSchedule>,
    run: &AgentRun,
) -> serde_json::Value {
    let Some(schedule) = schedule else {
        return json!({ "status": "skipped", "reason": "schedule no longer exists" });
    };

    let prefs: DeliveryPrefs =
        serde_json::from_value(schedule.delivery.clone()).unwrap_or_default();
    let (subject, body) = format_notification(schedule, run);
    let mut attempts: Vec<serde_json::Value> = Vec::new();

    if prefs.email {
        match state.resolve_owner_email(schedule.owner_user_id) {
            Some(to) => {
                let deliver = state.deliver_fn().clone();
                let email_subject = subject.clone();
                let email_body = body.clone();
                let to_addr = to.clone();
            let result = tokio::task::spawn_blocking(move || {
                    attempt_with_retries(&deliver, "email", &to_addr, &email_subject, &email_body)
                })
                .await;
                record_attempt(&mut attempts, "email", &to, result);
            }
            None => attempts.push(json!({
                "channel": "email", "status": "skipped",
                "detail": "owner e-mail could not be resolved",
            })),
        }
    }

    let sms_to = schedule
        .delivery
        .get("sms_to")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    if prefs.sms {
        if let Some(to) = sms_to {
            let deliver = state.deliver_fn().clone();
            let sms_subject = subject.clone();
            let sms_body = body.clone();
            let to_addr = to.clone();
            let result =
                tokio::task::spawn_blocking(move || {
                    attempt_with_retries(&deliver, "sms", &to, &sms_subject, &sms_body)
                })
                .await;
            record_attempt(&mut attempts, "sms", &to_addr, result);
        }
    }

    let channels_target = schedule
        .delivery
        .get("channels_target")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    for channel in &prefs.channels {
        let to = channels_target.clone().unwrap_or_else(|| channel.clone());
        let deliver = state.deliver_fn().clone();
        let channel_name = channel.clone();
        let subject = subject.clone();
        let body = body.clone();
        let to_addr = to.clone();
        let result = tokio::task::spawn_blocking(move || {
            attempt_with_retries(&deliver, &channel_name, &to_addr, &subject, &body)
        })
        .await;
        record_attempt(&mut attempts, channel, &to, result);
    }

    let overall = if attempts.iter().any(|a| a["status"] == "sent") {
        "sent"
    } else if attempts.is_empty() {
        "skipped"
    } else {
        "failed"
    };
    json!({ "status": overall, "attempts": attempts })
}

type DeliveryJoinResult = Result<Result<(), String>, tokio::task::JoinError>;

fn record_attempt(
    attempts: &mut Vec<serde_json::Value>,
    channel: &str,
    to: &str,
    result: DeliveryJoinResult,
) {
    let entry = match result {
        Ok(Ok(())) => json!({ "channel": channel, "to": mask_destination(to), "status": "sent" }),
        Ok(Err(e)) => json!({
            "channel": channel, "to": mask_destination(to),
            "status": "failed", "error": e,
        }),
        Err(e) => json!({
            "channel": channel, "to": mask_destination(to),
            "status": "failed", "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botautomation::delivery", None),
        }),
    };
    attempts.push(entry);
}

/// Masks the local-part of addresses in persisted status documents so
/// delivery metadata does not leak full recipient identities into logs.
fn mask_destination(to: &str) -> String {
    match to.split_once('@') {
        // An empty local part means a handle such as a Telegram or Matrix
        // nickname, not an email address; fall through to the opaque mask.
        Some((local, domain)) if !local.is_empty() => {
            let prefix = local.chars().take(2).collect::<String>();
            format!("{prefix}***@{domain}")
        }
        _ => {
            // Phone numbers and other opaque destinations keep their leading
            // significant digits; formatting such as a leading `+` is dropped so
            // the visible characters are the subscriber prefix.
            let significant: String = to.chars().filter(char::is_ascii_digit).collect();
            significant
                .chars()
                .take(4)
                .chain(std::iter::repeat('*'))
                .take(8)
                .collect()
        }
    }
}

/// Persists `delivery_status` onto the run row.
pub fn save_delivery_status(
    conn: &mut PgConnection,
    run_id: Uuid,
    doc: &serde_json::Value,
) -> Result<(), String> {
    use crate::schema::agent_runs::dsl::*;
    diesel::update(agent_runs.find(run_id))
        .set(delivery_status.eq(Some(doc.to_owned())))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| {
            tracing::error!("failed to persist delivery_status for run {run_id}: {e}");
            format!("persist delivery_status: {e}")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_schedule_covers_three_backoffs() {
        assert_eq!(RETRY_DELAYS_MS, [0, 1000, 2000, 4000]);
    }

    #[test]
    fn email_local_part_is_masked() {
        assert_eq!(mask_destination("owner@example.com"), "ow***@example.com");
        assert_eq!(mask_destination("+15551234567"), "1555****");
    }
}
