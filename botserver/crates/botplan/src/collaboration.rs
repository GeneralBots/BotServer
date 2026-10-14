use axum::{
    extract::{ws::WebSocketUpgrade, Path, Query},
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

pub type PlanChannels = Arc<tokio::sync::RwLock<HashMap<String, broadcast::Sender<PlanMessage>>>>;
pub type PlanPresence = Arc<tokio::sync::RwLock<HashMap<String, Vec<PlanUserPresence>>>>;
pub type PlanTyping = Arc<tokio::sync::RwLock<HashMap<String, Vec<PlanTypingIndicator>>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMessage {
    pub msg_type: String,
    pub plan_id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_color: String,
    pub task_id: Option<String>,
    pub content: Option<String>,
    pub position: Option<usize>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanUserPresence {
    pub user_id: String,
    pub user_name: String,
    pub user_color: String,
    pub current_view: String,
    pub selected_task: Option<String>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTypingIndicator {
    pub user_id: String,
    pub user_name: String,
    pub task_id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default, Deserialize)]
pub struct WsAuthQuery {
    pub token: String,
}

static PLAN_CHANNELS: std::sync::OnceLock<PlanChannels> = std::sync::OnceLock::new();
static PLAN_PRESENCE: std::sync::OnceLock<PlanPresence> = std::sync::OnceLock::new();
static PLAN_TYPING: std::sync::OnceLock<PlanTyping> = std::sync::OnceLock::new();

pub fn get_plan_channels() -> &'static PlanChannels {
    PLAN_CHANNELS.get_or_init(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())))
}
pub fn get_plan_presence() -> &'static PlanPresence {
    PLAN_PRESENCE.get_or_init(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())))
}
pub fn get_plan_typing() -> &'static PlanTyping {
    PLAN_TYPING.get_or_init(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())))
}

fn extract_user_from_token(token: &str) -> Option<(String, String)> {
    if token.is_empty() {
        return None;
    }
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_b64 = parts[1].replace('-', "+").replace('_', "/");
    let padding = (4 - payload_b64.len() % 4) % 4;
    let padded = format!("{}{}", payload_b64, "=".repeat(padding));
    let chars: Vec<u8> = padded.bytes().collect();
    let mut out = Vec::with_capacity(chars.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &c in &chars {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1u32 << bits) - 1;
        }
    }
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out) {
        let sub = v.get("sub").and_then(|x| x.as_str())
            .or_else(|| v.get("user_id").and_then(|x| x.as_str()))
            .or_else(|| v.get("email").and_then(|x| x.as_str()));
        let name = v.get("name").and_then(|x| x.as_str())
            .or_else(|| v.get("display_name").and_then(|x| x.as_str()))
            .or_else(|| v.get("preferred_username").and_then(|x| x.as_str()))
            .or_else(|| v.get("email").and_then(|x| x.as_str()));
        if let (Some(s), Some(n)) = (sub, name) {
            return Some((s.to_string(), n.to_string()));
        }
        if let Some(s) = sub {
            return Some((s.to_string(), s.to_string()));
        }
    }
    None
}

fn get_random_color() -> String {
    use rand::Rng;
    let colors = [
        "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7", "#DDA0DD",
        "#98D8C8", "#F7DC6F", "#BB8FCE", "#85C1E9", "#F1948A", "#82E0AA",
    ];
    let idx = rand::rng().random_range(0..colors.len());
    colors[idx].to_string()
}

pub async fn handle_plan_websocket(
    ws: WebSocketUpgrade,
    Path(plan_id): Path<String>,
    Query(q): Query<WsAuthQuery>,
) -> impl IntoResponse {
    let auth = extract_user_from_token(&q.token);
    ws.on_upgrade(move |socket| handle_plan_connection(socket, plan_id, auth))
}

async fn handle_plan_connection(
    socket: axum::extract::ws::WebSocket,
    plan_id: String,
    auth: Option<(String, String)>,
) {
    let (mut sender, mut receiver) = socket.split();

    let channels = get_plan_channels();
    let broadcast_tx = {
        let mut w = channels.write().await;
        w.entry(plan_id.clone())
            .or_insert_with(|| broadcast::channel(100).0)
            .clone()
    };
    let mut broadcast_rx = broadcast_tx.subscribe();

    let (user_id, user_name) = match auth {
        Some((id, name)) => (id, name),
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            (id.clone(), format!("Guest {}", &id[..8]))
        }
    };
    let user_id_for_send = user_id.clone();
    let user_color = get_random_color();

    {
        let mut presence = get_plan_presence().write().await;
        let users = presence.entry(plan_id.clone()).or_default();
        users.push(PlanUserPresence {
            user_id: user_id.clone(),
            user_name: user_name.clone(),
            user_color: user_color.clone(),
            current_view: "kanban".to_string(),
            selected_task: None,
            last_active: chrono::Utc::now(),
            status: "active".to_string(),
        });
    }

    let join_msg = PlanMessage {
        msg_type: "join".to_string(),
        plan_id: plan_id.clone(),
        user_id: user_id.clone(),
        user_name: user_name.clone(),
        user_color: user_color.clone(),
        task_id: None,
        content: None,
        position: None,
        timestamp: chrono::Utc::now(),
    };
    let _ = broadcast_tx.send(join_msg);

    let bc_tx_clone = broadcast_tx.clone();
    let uid_clone = user_id.clone();
    let pid_clone = plan_id.clone();
    let uname_clone = user_name.clone();
    let ucolor_clone = user_color.clone();

    let receive = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(axum::extract::ws::Message::Text(text)) => {
                    if let Ok(mut m) = serde_json::from_str::<PlanMessage>(&text) {
                        m.user_id = uid_clone.clone();
                        m.user_name = uname_clone.clone();
                        m.user_color = ucolor_clone.clone();
                        m.plan_id = pid_clone.clone();
                        m.timestamp = chrono::Utc::now();

                        match m.msg_type.as_str() {
                            "typing_start" => {
                                if let Some(tid) = m.task_id.clone() {
                                    let mut typing = get_plan_typing().write().await;
                                    let inds = typing.entry(pid_clone.clone()).or_default();
                                    inds.retain(|t| t.user_id != uid_clone);
                                    inds.push(PlanTypingIndicator {
                                        user_id: uid_clone.clone(),
                                        user_name: uname_clone.clone(),
                                        task_id: tid,
                                        started_at: chrono::Utc::now(),
                                    });
                                }
                            }
                            "typing_stop" => {
                                let mut typing = get_plan_typing().write().await;
                                if let Some(inds) = typing.get_mut(&pid_clone) {
                                    inds.retain(|t| t.user_id != uid_clone);
                                }
                            }
                            "view_change" => {
                                let mut presence = get_plan_presence().write().await;
                                if let Some(users) = presence.get_mut(&pid_clone) {
                                    for u in users.iter_mut() {
                                        if u.user_id == uid_clone {
                                            u.current_view = m.content.clone().unwrap_or_else(|| "kanban".to_string());
                                            u.last_active = chrono::Utc::now();
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }

                        let _ = bc_tx_clone.send(m);
                    }
                }
                Ok(axum::extract::ws::Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    let send = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if msg.user_id == user_id_for_send {
                continue;
            }
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let _ = tokio::join!(receive, send);

    {
        let mut presence = get_plan_presence().write().await;
        if let Some(users) = presence.get_mut(&plan_id) {
            users.retain(|u| u.user_id != user_id);
        }
    }
    {
        let mut typing = get_plan_typing().write().await;
        if let Some(inds) = typing.get_mut(&plan_id) {
            inds.retain(|t| t.user_id != user_id);
        }
    }

    let leave_msg = PlanMessage {
        msg_type: "leave".to_string(),
        plan_id: plan_id.clone(),
        user_id: user_id.clone(),
        user_name,
        user_color,
        task_id: None,
        content: None,
        position: None,
        timestamp: chrono::Utc::now(),
    };
    let _ = broadcast_tx.send(leave_msg);
}

pub async fn handle_get_plan_collaborators(
    Path(plan_id): Path<String>,
) -> impl IntoResponse {
    let presence = get_plan_presence().read().await;
    let users: Vec<&PlanUserPresence> = presence
        .get(&plan_id)
        .map(|v| v.iter().collect())
        .unwrap_or_default();
    Json(serde_json::json!({ "count": users.len(), "users": users }))
}

pub async fn handle_get_plan_presence(Path(plan_id): Path<String>) -> impl IntoResponse {
    let presence = get_plan_presence().read().await;
    let users = presence.get(&plan_id).cloned().unwrap_or_default();
    Json(serde_json::json!({ "users": users }))
}

pub async fn handle_get_plan_typing(Path(plan_id): Path<String>) -> impl IntoResponse {
    let typing = get_plan_typing().read().await;
    let indicators = typing.get(&plan_id).cloned().unwrap_or_default();
    let now = chrono::Utc::now();
    let active: Vec<&PlanTypingIndicator> = indicators
        .iter()
        .filter(|t| (now - t.started_at).num_seconds() < 5)
        .collect();
    Json(serde_json::json!({ "typing": active }))
}
