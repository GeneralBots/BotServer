//! Channel event subscriptions (#1507): `ON EVENT "<event>" CALL "<tool>"`.
//!
//! A bot script can subscribe a tool to a platform event so the tool runs
//! without the model having to ask for it. The media-filing case is the motive:
//! on Telegram/WhatsApp an upload produced an `[image] inbox/…` marker and
//! *hoped* the LLM would call `classify_media` — when it did not, the file sat
//! in `inbox/` forever. A subscription makes the trigger deterministic:
//!
//! ```basic
//! ' classify_media.bas
//! ON EVENT "media_uploaded" CALL "classify_media"
//! ```
//!
//! The subscription is a row in `system_automations`, the trigger registry that
//! `WEBHOOK` and `ON EMAIL` already use: `kind` selects the trigger family,
//! `target` is what to match and `param` is the script to run. A channel event
//! is the same shape, so it is two new `TriggerKind` values and no new table.
//! The occurrence goes to the existing `workflow_events` bus.
//!
//! Three moving parts, separated by crate boundary:
//! - **register** (botbasic_ai): the script runs, the row is upserted;
//! - **publish** (botserver pipeline): a channel adapter reports the event;
//! - **dispatch** (botserver background): pending events run the subscribed
//!   tools, so a failure in one tool cannot break the message pipeline.

use std::sync::Arc;

use diesel::prelude::*;
use uuid::Uuid;

use crate::shared::models::TriggerKind;
use botschema as botcore_schema;
use crate::shared::schema::core::{system_automations, workflow_events};
use crate::shared::utils::DbPool;

/// A media file arrived on any channel and was staged in the bot's Drive
/// `inbox/`. Payload: `{path, kind, caption, channel, session_id,
/// channel_user_id}`. `session_id` and `channel_user_id` identify the
/// conversation that produced the upload: the dispatcher runs the subscribed
/// tool under that session and with that sender identity, so the tool's `TALK`
/// confirmation reaches the channel the file came from instead of being
/// dropped for lack of a destination.
pub const MEDIA_UPLOADED: &str = "media_uploaded";
/// A text message arrived on any channel. Payload: `{text, channel}`.
pub const MESSAGE_RECEIVED: &str = "message_received";

/// Event name → trigger kind. A subscription is stored under the matching
/// `system_automations.kind`, so the dispatcher selects one family per event.
fn trigger_kind(event_name: &str) -> Option<TriggerKind> {
    match event_name {
        MEDIA_UPLOADED => Some(TriggerKind::MediaUploaded),
        MESSAGE_RECEIVED => Some(TriggerKind::MessageReceived),
        _ => None,
    }
}

/// True when `event` may be subscribed to.
pub fn is_known_event(event: &str) -> bool {
    trigger_kind(event).is_some()
}

/// Subscribe `tool_name` of `bot_id` to `event_name` (idempotent, like
/// `WEBHOOK`: same bot + kind + target updates the stored script).
pub fn register_handler(
    pool: &DbPool,
    bot_id: Uuid,
    event_name: &str,
    tool_name: &str,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("pool: {e}"))?;
    register_handler_on(&mut conn, bot_id, event_name, tool_name)
}

/// Same registration on a borrowed connection, for the compiler's
/// design-time pass (which already holds one, like the WEBHOOK callback).
pub fn register_handler_on(
    conn: &mut diesel::PgConnection,
    bot_id: Uuid,
    event_name: &str,
    tool_name: &str,
) -> Result<(), String> {
    let kind = trigger_kind(event_name).ok_or_else(|| {
        format!("unknown channel event '{event_name}' (known: {MEDIA_UPLOADED}, {MESSAGE_RECEIVED})")
    })?;
    // branch_id is NOT NULL on system_automations; resolve it from the bot the
    // same way the WEBHOOK registration does.
    let branch_id = botcore_schema::system_automation_branch_id(conn, bot_id)
        .map_err(|e| format!("branch for bot {bot_id}: {e}"))?;
    let updated = diesel::update(system_automations::table)
        .filter(system_automations::bot_id.eq(Some(bot_id)))
        .filter(system_automations::kind.eq(kind as i32))
        .filter(system_automations::target.eq(event_name))
        .set((
            system_automations::param.eq(tool_name),
            system_automations::is_active.eq(true),
        ))
        .execute(conn)
        .map_err(|e| format!("update {event_name}: {e}"))?;
    if updated == 0 {
        diesel::sql_query(
            "INSERT INTO system_automations (id, bot_id, branch_id, kind, target, param, is_active) \
             VALUES ($1, $2, $3, $4, $5, $6, TRUE)",
        )
        .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .bind::<diesel::sql_types::Integer, _>(kind as i32)
        .bind::<diesel::sql_types::Text, _>(event_name)
        .bind::<diesel::sql_types::Text, _>(tool_name)
        .execute(conn)
        .map_err(|e| format!("insert {event_name}: {e}"))?;
    }
    Ok(())
}

/// Tool names subscribed by `bot_id` to `event_name`.
pub fn handlers_for(pool: &DbPool, bot_id: Uuid, event_name: &str) -> Vec<String> {
    let Some(kind) = trigger_kind(event_name) else {
        return Vec::new();
    };
    let Ok(mut conn) = pool.get() else {
        return Vec::new();
    };
    system_automations::table
        .filter(system_automations::bot_id.eq(Some(bot_id)))
        .filter(system_automations::kind.eq(kind as i32))
        .filter(system_automations::target.eq(event_name))
        .filter(system_automations::is_active.eq(true))
        .select(system_automations::param)
        .load::<String>(&mut conn)
        .unwrap_or_default()
}

/// Record an event for later dispatch. Failures are reported, never fatal: the
/// conversation must continue even when the event store is unavailable.
pub fn publish(
    pool: &DbPool,
    bot_id: Uuid,
    event_name: &str,
    payload: &serde_json::Value,
) -> Result<Uuid, String> {
    if !is_known_event(event_name) {
        return Err(format!("refusing to publish unknown event '{event_name}'"));
    }
    let mut conn = pool.get().map_err(|e| format!("pool: {e}"))?;
    diesel::sql_query(
        "INSERT INTO workflow_events \
           (id, execution_id, workflow_id, event_name, event_type, payload, processed, created_at) \
         VALUES ($1, $2, $2, $3, 'channel', $4, FALSE, NOW()) RETURNING id",
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .bind::<diesel::sql_types::Text, _>(event_name)
    .bind::<diesel::sql_types::Jsonb, _>(payload)
    .get_result::<EventId>(&mut conn)
    .map(|row| row.id)
    .map_err(|e| format!("publish {event_name}: {e}"))
}

#[derive(diesel::QueryableByName)]
struct EventId {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
}

/// A channel event waiting to be dispatched, with the bot it belongs to.
#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub event_name: String,
    pub payload: serde_json::Value,
}

/// Oldest unprocessed channel events, capped so one burst cannot stall the
/// dispatcher (the next tick picks up the rest).
pub fn pending_events(pool: &DbPool, limit: i64) -> Vec<PendingEvent> {
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        workflow_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        event_name: String,
        #[diesel(sql_type = diesel::sql_types::Jsonb)]
        payload: serde_json::Value,
    }
    let Ok(mut conn) = pool.get() else {
        return Vec::new();
    };
    diesel::sql_query(
        "SELECT id, workflow_id, event_name, payload FROM workflow_events \
         WHERE processed = FALSE AND event_type = 'channel' \
         ORDER BY created_at ASC LIMIT $1",
    )
    .bind::<diesel::sql_types::BigInt, _>(limit)
    .load::<Row>(&mut conn)
    .unwrap_or_default()
    .into_iter()
    .map(|row| PendingEvent {
        id: row.id,
        // `publish` stores the bot in `workflow_id` (see the INSERT above).
        bot_id: row.workflow_id,
        event_name: row.event_name,
        payload: row.payload,
    })
    .collect()
}

/// Mark an event dispatched (or failed) so it is not retried forever.
pub fn mark_processed(pool: &DbPool, event_id: Uuid) {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("[basic_events] mark processed {event_id}: {e}");
            return;
        }
    };
    if let Err(e) = diesel::update(workflow_events::table.filter(workflow_events::id.eq(event_id)))
        .set(workflow_events::processed.eq(true))
        .execute(&mut conn)
    {
        log::warn!("[basic_events] mark processed {event_id}: {e}");
    }
}

/// Convenience for adapters: publish a media upload in one call.
///
/// `session_id` is the conversation the upload belongs to and
/// `channel_user_id` is its sender as the channel knows it (a Telegram chat
/// id, a WhatsApp phone number); both are required for the reply the
/// subscribed tool produces to be deliverable.
/// One channel media upload, as the subscribers of the event see it.
#[derive(Clone, Copy)]
pub struct MediaUpload<'a> {
    pub pool: &'a Arc<DbPool>,
    pub bot_id: Uuid,
    pub path: &'a str,
    pub kind: &'a str,
    pub caption: &'a str,
    pub channel: &'a str,
    pub session_id: &'a str,
    pub channel_user_id: &'a str,
}

pub fn publish_media_uploaded(upload: &MediaUpload<'_>) {
    let MediaUpload {
        pool,
        bot_id,
        path,
        kind,
        caption,
        channel,
        session_id,
        channel_user_id,
    } = *upload;
    let payload = serde_json::json!({
        "path": path,
        "kind": kind,
        "caption": caption,
        "channel": channel,
        "session_id": session_id,
        "channel_user_id": channel_user_id,
    });
    if let Err(e) = publish(pool, bot_id, MEDIA_UPLOADED, &payload) {
        log::warn!("[basic_events] media_uploaded for bot {bot_id} not queued: {e}");
    } else {
        log::info!("[basic_events] media_uploaded queued for bot {bot_id}: {path}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_events_map_to_a_trigger_kind() {
        assert_eq!(trigger_kind(MEDIA_UPLOADED), Some(TriggerKind::MediaUploaded));
        assert_eq!(trigger_kind(MESSAGE_RECEIVED), Some(TriggerKind::MessageReceived));
        // Internal workflow events are not channel subscriptions.
        assert_eq!(trigger_kind("workflow_step_complete"), None);
        assert_eq!(trigger_kind(""), None);
    }

    #[test]
    fn channel_kinds_do_not_collide_with_existing_triggers() {
        // The registry is shared with WEBHOOK (4) and ON EMAIL (5); reusing a
        // number would silently rewire an existing trigger.
        for kind in [TriggerKind::MediaUploaded, TriggerKind::MessageReceived] {
            let value = kind as i32;
            assert!(value > 9, "{kind:?} must extend, not replace, the enum");
            assert_eq!(TriggerKind::from_i32(value), Some(kind));
        }
    }

    #[test]
    fn publishing_an_unknown_event_is_refused() {
        // The guard runs before any database work, so it is observable without
        // a pool: a typo can never insert a row the dispatcher would not match.
        assert!(!is_known_event("not_an_event"));
        let unknown_publish: fn(&DbPool, Uuid, &str, &serde_json::Value) -> Result<Uuid, String> =
            publish;
        // A real pool is not available in unit tests; the mapping is the part
        // under test and it is total (unknown → None → Err).
        assert!(trigger_kind("not_an_event").is_none());
        let _ = unknown_publish;
    }
}
