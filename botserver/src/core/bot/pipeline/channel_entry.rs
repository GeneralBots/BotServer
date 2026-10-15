//! Channel entry point for the message pipeline (#939 D split).
//!
//! `run_pipeline_for_channel` adapts a structured [`UserMessage`] arriving
//! from any channel into the raw JSON envelope consumed by
//! `process_message_internal`, resolving the bot name to its UUID first.
//! Split out of `exec.rs` to keep every pipeline file under the 450-line
//! repository limit; behavior is unchanged.

use std::sync::Arc;

use botcore::shared::state::AppState;
use uuid::Uuid;

use super::exec::process_message_internal;
use super::sink::ChannelSink;
use super::types::{PipelineResult, SessionIdentity};

pub async fn run_pipeline_for_channel(
    state: &Arc<AppState>,
    msg: &botlib::models::UserMessage,
    sink: &dyn ChannelSink,
) -> PipelineResult<()> {
    // `msg.bot_id` carries a bot NAME on some channels and a bot UUID on
    // others (the Telegram webhook fills it with the UUID). Everything
    // downstream — tool loading (`get_session_tools` looks up
    // `bots.name`), work-dir paths, prompt text — expects the NAME, so
    // normalize here once instead of patching each consumer.
    let mut bot_name = msg.bot_id.clone();
    if Uuid::parse_str(&bot_name).is_ok() {
        if let Some(resolved) = resolve_bot_name(&state.conn, &bot_name).await {
            bot_name = resolved;
        }
    }
    let user_text = msg.content.clone();
    let session_id = Uuid::parse_str(&msg.session_id).unwrap_or_else(|_| Uuid::new_v4());
    // The sink reports its TRANSPORT ("mpsc", "web"), not the channel the
    // message came from, and Telegram/WhatsApp both arrive over the mpsc
    // transport — keying on it gave every such sender the same identity.
    // `msg.channel` is the real channel.
    let channel_key = if msg.channel.trim().is_empty() {
        sink.channel_type().to_string()
    } else {
        msg.channel.trim().to_lowercase()
    };
    let user_id = channel_user_uuid(&channel_key, &msg.user_id);

    let bot_uuid = resolve_bot_uuid(&state.conn, &bot_name).await;

    // Channel event subscriptions (#1507): an upload published here runs the
    // tools a script subscribed with `ON EVENT "media_uploaded" CALL "…"`,
    // independently of whether the model decides to call them. The marker
    // `[image] inbox/…` is what tells us a file was staged.
    //
    // The event carries the conversation identity (`session_id`) and the
    // sender as the channel knows it (`msg.user_id`, a Telegram chat id for
    // example): the dispatcher runs the subscribed tool under that pair, so
    // the tool's confirmation is addressed to the conversation that uploaded
    // the file instead of being dropped.
    let reply_target = ChannelReplyTarget {
        session_id: session_id.to_string(),
        user_id: msg.user_id.clone(),
    };
    publish_channel_event(state, bot_uuid, &user_text, &channel_key, &reply_target);

    let response_key = format!("{}_{}", session_id, Uuid::new_v4());
    let (tx_internal, mut rx_internal) =
        tokio::sync::mpsc::channel::<botlib::models::BotResponse>(100);
    {
        let mut channels = state.response_channels.lock().await;
        channels.insert(response_key.clone(), tx_internal);
    }

    let json_msg = serde_json::json!({
        "text": user_text,
        "content": user_text,
        "message_type": i32::from(msg.message_type),
    })
    .to_string();

    let mut start_bas_ran = false;
    let identity = SessionIdentity {
        sink,
        state,
        bot_uuid,
        session_id,
        user_id,
        bot_name: &bot_name,
    };
    let result = process_message_internal(&identity, &mut rx_internal, &mut start_bas_ran, &json_msg)
        .await;

    {
        let mut channels = state.response_channels.lock().await;
        channels.remove(&response_key);
    }

    result
}

/// Where a reply to a channel event must go: the conversation it belongs to
/// and the sender identity the channel adapter expects.
struct ChannelReplyTarget {
    session_id: String,
    user_id: String,
}

/// Report the channel events a bot can subscribe to.
///
/// Media markers are `[image] inbox/…`, `[document] …`, `[voice] …`,
/// `[audio] …`, `[video] …` and `[sticker] …` (see `bottelegram::media`);
/// the file is already in the bot's Drive `inbox/` at that point. A plain
/// message is only published as `message_received` — every inbound message
/// would otherwise wake every subscriber on every keystroke-sized text.
fn publish_channel_event(
    state: &Arc<AppState>,
    bot_uuid: uuid::Uuid,
    user_text: &str,
    channel: &str,
    reply_target: &ChannelReplyTarget,
) {
    use botcore::shared::basic_events;
    if bot_uuid.is_nil() {
        return;
    }
    const MARKERS: [(&str, &str); 6] = [
        ("[image]", "image"),
        ("[document]", "document"),
        ("[voice]", "audio"),
        ("[audio]", "audio"),
        ("[video]", "video"),
        ("[sticker]", "image"),
    ];
    for (marker, kind) in MARKERS {
        let Some(start) = user_text.find(marker) else {
            continue;
        };
        let rest = &user_text[start + marker.len()..];
        let path = rest
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches(',')
            .to_string();
        if path.is_empty() {
            continue;
        }
        // The caption follows the path on the next line (bottelegram::media
        // and botwhatsapp::media both emit "[kind] path\ncaption"); the web
        // producer puts the user's typed text BEFORE its marker. Take the
        // after-path tail when there is one, otherwise the leading text.
        let after_path = rest[path.len()..].trim();
        let caption = if after_path.is_empty() {
            user_text[..start].trim().to_string()
        } else {
            after_path.to_string()
        };
        let pool = Arc::new(state.conn.clone());
        basic_events::publish_media_uploaded(&basic_events::MediaUpload {
            pool: &pool,
            bot_id: bot_uuid,
            path: &path,
            kind,
            caption: &caption,
            channel,
            session_id: &reply_target.session_id,
            channel_user_id: &reply_target.user_id,
        });
        return;
    }
    let text = user_text.trim();
    if !text.is_empty() && !text.starts_with('[') {
        let payload = serde_json::json!({ "text": text, "channel": channel });
        if let Err(e) = basic_events::publish(&state.conn, bot_uuid, basic_events::MESSAGE_RECEIVED, &payload) {
            log::debug!("message_received not queued for bot {bot_uuid}: {e}");
        }
    }
}

/// Stable identity for a channel sender, used for RBAC and for `rbac_api_permissions`.
///
/// Channel user ids are not UUIDs (a Telegram chat id is `"6676512312"`, a
/// WhatsApp id is a phone number), and parsing them used to fall back to
/// `Uuid::nil()`. Every channel user therefore shared one identity, so
/// `resolve_user_role` could never match a group and every admin-only
/// command — `tasks.autotask.create` among them — was unreachable from chat,
/// Telegram and WhatsApp. Deriving a v5 UUID per `channel:external_id` keeps
/// each sender distinct and lets an administrator grant a role to exactly
/// that sender.
pub fn channel_user_uuid(channel: &str, external_user_id: &str) -> Uuid {
    let trimmed = external_user_id.trim();
    if trimmed.is_empty() {
        return Uuid::nil();
    }
    if let Ok(parsed) = Uuid::parse_str(trimmed) {
        return parsed;
    }
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("{channel}:{trimmed}").as_bytes(),
    )
}

/// Reverse lookup for callers that pass a bot UUID where a name is
/// expected. Returns `None` when the value is not a known bot id.
async fn resolve_bot_name(
    pool: &botcore::shared::utils::DbPool,
    bot_uuid: &str,
) -> Option<String> {
    let uuid = Uuid::parse_str(bot_uuid).ok()?;
    use diesel::prelude::*;
    if let Ok(mut conn) = pool.get_timeout(std::time::Duration::from_secs(3)) {
        #[derive(diesel::QueryableByName)]
        struct BotName {
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
        }
        diesel::sql_query("SELECT name FROM bots WHERE id = $1 LIMIT 1")
            .bind::<diesel::sql_types::Uuid, _>(uuid)
            .get_result::<BotName>(&mut conn)
            .ok()
            .map(|r| r.name)
    } else {
        None
    }
}

async fn resolve_bot_uuid(pool: &botcore::shared::utils::DbPool, bot_name: &str) -> uuid::Uuid {
    if let Ok(uuid) = uuid::Uuid::parse_str(bot_name) {
        return uuid;
    }
    use diesel::RunQueryDsl;
    if let Ok(mut conn) = pool.get_timeout(std::time::Duration::from_secs(3)) {
        #[derive(diesel::QueryableByName)]
        struct BotId {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: uuid::Uuid,
        }
        diesel::sql_query("SELECT id FROM bots WHERE name = $1 AND is_active = true LIMIT 1")
            .bind::<diesel::sql_types::Text, _>(bot_name)
            .get_result::<BotId>(&mut conn)
            .ok()
            .map(|r| r.id)
            .unwrap_or_default()
    } else {
        uuid::Uuid::nil()
    }
}

#[cfg(test)]
mod tests {
    use super::channel_user_uuid;

    #[test]
    fn non_uuid_channel_ids_get_a_stable_per_sender_uuid() {
        let a = channel_user_uuid("telegram", "6676512312");
        let b = channel_user_uuid("telegram", "6676512312");
        let other = channel_user_uuid("telegram", "999");
        let whatsapp = channel_user_uuid("whatsapp", "6676512312");
        assert_eq!(a, b, "same sender must keep one identity across turns");
        assert_ne!(a, other, "different senders must not collide");
        assert_ne!(a, whatsapp, "the same id on another channel is another user");
        assert_ne!(a, uuid::Uuid::nil(), "must not collapse to the nil user");
    }

    /// The transport label is not a channel: Telegram and WhatsApp both report
    /// "mpsc", so keying on it merged their senders.
    #[test]
    fn the_transport_label_is_not_used_as_the_channel() {
        let telegram = channel_user_uuid("telegram", "6676512312");
        assert_ne!(
            telegram,
            channel_user_uuid("mpsc", "6676512312"),
            "the mpsc transport must not stand in for the channel"
        );
    }

    #[test]
    fn real_uuids_and_blank_ids_are_passed_through() {
        let raw = uuid::Uuid::new_v4();
        assert_eq!(channel_user_uuid("web", &raw.to_string()), raw);
        assert_eq!(channel_user_uuid("telegram", "   "), uuid::Uuid::nil());
    }
}
