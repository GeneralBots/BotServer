use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};
use uuid::Uuid;

use crate::responses::ExportProgressEvent;
use crate::routes::AppState;

static GLOBAL_BROADCASTER: std::sync::OnceLock<Arc<ExportProgressBroadcaster>> =
    std::sync::OnceLock::new();

pub struct ExportProgressBroadcaster {
    tx: broadcast::Sender<ExportProgressEvent>,
}

impl ExportProgressBroadcaster {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    pub fn global() -> Arc<Self> {
        GLOBAL_BROADCASTER
            .get_or_init(|| Arc::new(Self::new()))
            .clone()
    }

    pub fn sender(&self) -> broadcast::Sender<ExportProgressEvent> {
        self.tx.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ExportProgressEvent> {
        self.tx.subscribe()
    }

    pub fn send(&self, event: ExportProgressEvent) {
        if let Err(e) = self.tx.send(event) {
            warn!("No active WebSocket listeners: {e}");
        }
    }
}

impl Default for ExportProgressBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn export_progress_websocket(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
    Path(export_id): Path<Uuid>,
) -> impl IntoResponse {
    info!("WebSocket connection request for export: {export_id}");
    ws.on_upgrade(move |socket| handle_export_websocket(socket, export_id))
}

async fn handle_export_websocket(socket: WebSocket, export_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    info!("WebSocket connected for export: {export_id}");

    let welcome = serde_json::json!({
        "type": "connected",
        "export_id": export_id.to_string(),
        "message": "Connected to export progress stream",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if let Err(e) = sender
        .send(Message::Text(welcome.to_string()))
        .await
    {
        log::error!("Failed to send welcome message: {e}");
        return;
    }

    let broadcaster = ExportProgressBroadcaster::global();
    let mut progress_rx = broadcaster.subscribe();

    let export_id_for_recv = export_id;

    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => {
                    info!("WebSocket close requested for export: {export_id_for_recv}");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    info!("Received ping for export: {export_id_for_recv}");
                }
                Ok(Message::Text(text)) => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if json.get("type").and_then(|v| v.as_str()) == Some("ping") {
                            info!("Client ping received");
                        }
                    }
                }
                Err(e) => {
                    log::error!("WebSocket receive error: {e}");
                    break;
                }
                _ => {}
            }
        }
    });

    loop {
        tokio::select! {
            result = progress_rx.recv() => {
                match result {
                    Ok(event) => {
                        if event.export_id == export_id {
                            let json = serde_json::json!({
                                "type": "progress",
                                "export_id": event.export_id.to_string(),
                                "project_id": event.project_id.to_string(),
                                "status": event.status,
                                "progress": event.progress,
                                "message": event.message,
                                "output_url": event.output_url,
                                "gbdrive_path": event.gbdrive_path,
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            });

                            if let Err(e) = sender.send(Message::Text(json.to_string())).await {
                                log::error!("Failed to send progress update: {e}");
                                break;
                            }

                            if event.status == "completed" || event.status == "failed" {
                                let final_msg = serde_json::json!({
                                    "type": "finished",
                                    "export_id": event.export_id.to_string(),
                                    "status": event.status,
                                    "output_url": event.output_url,
                                    "gbdrive_path": event.gbdrive_path
                                });

                                let _ = sender.send(Message::Text(final_msg.to_string())).await;
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket lagged behind by {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Progress broadcast channel closed");
                        break;
                    }
                }
            }

            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                let heartbeat = serde_json::json!({
                    "type": "heartbeat",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                if let Err(e) = sender.send(Message::Text(heartbeat.to_string())).await {
                    log::error!("Failed to send heartbeat: {e}");
                    break;
                }
            }
        }
    }

    recv_task.abort();
    info!("WebSocket disconnected for export: {export_id}");
}

pub fn broadcast_export_progress(
    export_id: Uuid,
    project_id: Uuid,
    status: &str,
    progress: i32,
    message: Option<String>,
    output_url: Option<String>,
    gbdrive_path: Option<String>,
) {
    let event = ExportProgressEvent {
        export_id,
        project_id,
        status: status.to_string(),
        progress,
        message,
        output_url,
        gbdrive_path,
    };

    ExportProgressBroadcaster::global().send(event);
}
