use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};

use botcore::shared::state::AppState;

/// GET /ws/task-progress — streams AutoTask progress events to clients.
///
/// botui proxies `/ws/task-progress[/:task_id]` here (see
/// `botui/src/ui_server/ws.rs::ws_task_progress_proxy`). Events come from the
/// shared `task_progress_broadcast` channel fed by AutoTask producers
/// (`AgentExecutor::broadcast_step`, terminal output, LLM streaming, ...).
/// When a task id is present in the path only events for that task are sent.
pub async fn task_progress_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| task_progress_ws_loop(socket, state, None))
}

/// GET /ws/task-progress/:task_id — same stream, filtered to one task.
pub async fn task_progress_ws_with_id(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Response {
    ws.on_upgrade(move |socket| task_progress_ws_loop(socket, state, Some(task_id)))
}

async fn task_progress_ws_loop(
    socket: WebSocket,
    state: Arc<AppState>,
    task_filter: Option<String>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Hello frame — the UI treats `{"type":"connected"}` as ready (see
    // botui/ui/suite/vibe/vibe-websocket.js).
    let hello = serde_json::json!({ "type": "connected" });
    if sender
        .send(Message::Text(hello.to_string()))
        .await
        .is_err()
    {
        return;
    }

    let mut events_rx = match &state.task_progress_broadcast {
        Some(tx) => tx.subscribe(),
        None => {
            log::warn!("[task-progress-ws] no broadcast channel configured");
            return;
        }
    };

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if text == "ping"
                            && sender.send(Message::Text("pong".into())).await.is_err() {
                                break;
                            }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        log::debug!("[task-progress-ws] client error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            event = events_rx.recv() => {
                match event {
                    Ok(evt) => {
                        if let Some(filter) = &task_filter {
                            if &evt.task_id != filter {
                                continue;
                            }
                        }
                        match serde_json::to_string(&evt) {
                            Ok(payload) => {
                                if sender.send(Message::Text(payload)).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => log::warn!("[task-progress-ws] serialize failed: {e}"),
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("[task-progress-ws] lagged {n} events");
                    }
                    Err(_) => break,
                }
            }
        }
    }
    log::info!("[task-progress-ws] connection closed");
}

/// Convert an AutoTask-domain progress event into the core event type and
/// push it onto the shared broadcast channel consumed by /ws/task-progress.
///
/// The two structs are field-identical but live in different crates
/// (botautotask has no dependency on botcore), so the mapping is explicit —
/// the compiler forces an update here if either side gains fields.
pub fn forward_autotask_event(app_state: &AppState, event: botautotask::types::TaskProgressEvent) {
    let core_event = botcore::shared::state::TaskProgressEvent {
        event_type: event.event_type.clone(),
        task_id: event.task_id.clone(),
        step: event.step.clone(),
        message: event.message.clone(),
        progress: event.progress,
        total_steps: event.total_steps,
        current_step: event.current_step,
        timestamp: event.timestamp.clone(),
        details: event.details.clone(),
        error: event.error.clone(),
        activity: event.activity.map(|a| botcore::shared::state::AgentActivity {
            phase: a.phase.clone(),
            items_processed: a.items_processed,
            items_total: a.items_total,
            speed_per_min: a.speed_per_min,
            eta_seconds: a.eta_seconds,
            current_item: a.current_item.clone(),
            bytes_processed: a.bytes_processed,
            tokens_used: a.tokens_used,
            files_created: a.files_created.clone(),
            tables_created: a.tables_created.clone(),
            log_lines: a.log_lines.clone(),
        }),
        text: event.text,
    };
    app_state.broadcast_task_progress(core_event);
}
