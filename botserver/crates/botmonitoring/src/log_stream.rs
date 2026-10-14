use crate::real_time::MonitoringMessage;
use crate::MonitoringState;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::json;
use std::sync::Arc;

pub async fn ws_logs<S: MonitoringState>(
    State(state): State<Arc<S>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_log_socket(socket, state))
}

async fn handle_log_socket<S: MonitoringState>(mut socket: WebSocket, state: Arc<S>) {
    let collector = state.metrics_collector();
    let mut rx = collector.subscribe();

    let welcome = json!({
        "id": "monitoring-0",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "level": "info",
        "service": "monitoring",
        "message": "Monitoring log stream connected",
    });
    if socket
        .send(Message::Text(welcome.to_string()))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(MonitoringMessage::AlertFired { alert }) => {
                        let entry = json!({
                            "id": format!("alert-{}", alert.id),
                            "timestamp": alert.started_at.to_rfc3339(),
                            "level": "error",
                            "service": "alerts",
                            "message": format!("[{}] {}", alert.rule_name, alert.message),
                            "context": {
                                "severity": format!("{:?}", alert.severity),
                                "metric": alert.metric_name,
                                "value": alert.metric_value,
                            },
                        });
                        if socket.send(Message::Text(entry.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Ok(MonitoringMessage::HealthUpdate { health }) => {
                        let status = format!("{:?}", health.status);
                        let level = if health.status == crate::HealthStatus::Healthy {
                            "info"
                        } else {
                            "warn"
                        };
                        let entry = json!({
                            "id": format!("health-{}", health.last_check.timestamp_millis()),
                            "timestamp": health.last_check.to_rfc3339(),
                            "level": level,
                            "service": "monitoring",
                            "message": format!(
                                "System health: {} (CPU {:.1}%, Mem {:.1}%, Disk {:.1}%)",
                                status, health.cpu_usage_percent, health.memory_usage_percent, health.disk_usage_percent
                            ),
                            "context": {
                                "cpu": health.cpu_usage_percent,
                                "memory": health.memory_usage_percent,
                                "disk": health.disk_usage_percent,
                                "latency_ms": health.average_latency_ms,
                            },
                        });
                        if socket.send(Message::Text(entry.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Ok(MonitoringMessage::AlertResolved { alert_id }) => {
                        let entry = json!({
                            "id": format!("resolve-{}", alert_id),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "level": "info",
                            "service": "alerts",
                            "message": format!("Alert {alert_id} resolved"),
                        });
                        if socket.send(Message::Text(entry.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    }
}
