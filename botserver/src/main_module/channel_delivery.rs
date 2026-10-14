//! Delivery of bot replies that have no live response channel.
//!
//! A channel reply normally travels through `AppState::response_channels`: the
//! pipeline registers a sender for the conversation while the inbound request
//! runs and the channel adapter forwards whatever arrives on it. A tool
//! triggered by `ON EVENT` runs in the background, so its `TALK` reaches this
//! layer after that registration is gone and used to be dropped with
//! `send_message: NO channel for session …` — the media-filing confirmation,
//! for example, never reached Telegram even though the file was filed.
//!
//! This module closes that gap for the channels whose adapter can address a
//! recipient on its own: `BotResponse.channel` selects the adapter and
//! `BotResponse.user_id` is the recipient that adapter expects (a Telegram chat
//! id for `TALK` coming from a channel event). Channels without an
//! addressable adapter keep the previous behavior — a warning and no delivery
//! — instead of silently pretending to have sent the message.

use botcore::shared::state::AppState;
use botlib::models::BotResponse;
use std::sync::Arc;

/// Hands `response` to the adapter of its channel.
///
/// Returns `false` when the channel has no adapter that can address a
/// recipient on its own, in which case the caller reports the drop.
#[must_use]
pub fn deliver_via_channel(state: &Arc<AppState>, response: &BotResponse) -> bool {
    match response.channel.as_str() {
        "telegram" => deliver_telegram(state, response),
        _ => false,
    }
}

/// Telegram addresses a reply by chat id, the value the channel event carries
/// as `channel_user_id` and `TALK` copies into `BotResponse.user_id`.
#[cfg(feature = "telegram")]
fn deliver_telegram(state: &Arc<AppState>, response: &BotResponse) -> bool {
    let chat_id = response.user_id.trim().to_string();
    if chat_id.is_empty() {
        log::warn!(
            "channel delivery: telegram reply for session {} has no chat id",
            response.session_id
        );
        return false;
    }

    let bot_id = match uuid::Uuid::parse_str(response.bot_id.trim()) {
        Ok(bot_id) => bot_id,
        Err(e) => {
            log::warn!(
                "channel delivery: telegram reply for unknown bot '{}': {e}",
                response.bot_id
            );
            return false;
        }
    };

    let pool = Arc::new(state.conn.clone());
    let get_config = crate::main_module::routes::channel_support::make_channel_config_reader(state);
    let message = response.clone();

    // Callers reach `send_bot_response` from a blocking context, so the request runs
    // on its own current-thread runtime and never blocks the script.
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => {
                log::warn!("channel delivery: telegram runtime unavailable: {e}");
                return;
            }
        };

        let adapter = bottelegram::TelegramAdapter::new(pool, bot_id, get_config);
        let session_id = message.session_id.clone();
        let sent = runtime
            .block_on(bottelegram::ChannelAdapter::send_bot_response(&adapter, message));

        match sent {
            Ok(()) => log::info!(
                "channel delivery: telegram reply sent to chat {chat_id} for session {session_id}"
            ),
            Err(e) => log::warn!(
                "channel delivery: telegram reply to chat {chat_id} failed for session {session_id}: {e}"
            ),
        }
    });
    true
}

/// Builds without the Telegram channel: the adapter is not compiled in, so the
/// reply stays in the log the way it did before this module existed.
#[cfg(not(feature = "telegram"))]
fn deliver_telegram(state: &Arc<AppState>, response: &BotResponse) -> bool {
    log::debug!(
        "channel delivery: telegram support not compiled in; reply for session {} not delivered",
        response.session_id
    );
    let _ = state;
    false
}
