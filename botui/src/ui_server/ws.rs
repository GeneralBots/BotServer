use axum::{
    extract::{
        ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
        OriginalUri, Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::Deserialize;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite,
    tungstenite::protocol::Message as TungsteniteMessage,
};

use crate::shared::AppState;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub bot_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OptionalWsQuery {
    task_id: Option<String>,
}

/// Generic WebSocket proxy for backend API WS endpoints (/api/terminal/ws,
/// /api/browser/.../ws, etc.). reqwest-based proxy_api cannot tunnel WS
/// upgrades, so API WS routes must be extened here with a real proxy that
/// preserves the original path + query (id, token, ...).
pub async fn api_ws_proxy(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
) -> impl IntoResponse {
    let forwarded_path = uri.path().to_string();
    let forwarded_query = uri.query().map_or_else(String::new, |q| format!("?{q}"));
    ws.on_upgrade(move |socket| {
        handle_api_ws_proxy(socket, state, forwarded_path, forwarded_query)
    })
}

async fn handle_api_ws_proxy(
    client_socket: WebSocket,
    state: AppState,
    path: String,
    query: String,
) {
    let backend_url = format!(
        "{}{path}{query}",
        state
            .client
            .base_url()
            .replace("https://", "wss://")
            .replace("http://", "ws://"),
    );

    info!("Proxying API WebSocket to: {backend_url}");

    let backend_socket = if backend_url.starts_with("wss://") {
        let mut tls_builder = native_tls::TlsConnector::builder();
        let ca_path = botlib::security::ca_cert_path();
        if let Ok(pem) = std::fs::read(&ca_path) {
            match native_tls::Certificate::from_pem(&pem) {
                Ok(cert) => {
                    tls_builder.add_root_certificate(cert);
                }
                Err(e) => warn!("Invalid platform CA PEM at {ca_path}: {e}"),
            }
        }
        let Ok(tls_connector) = tls_builder.build()
        else {
            error!("Failed to build TLS connector for API WebSocket proxy");
            return;
        };
        let connector = tokio_tungstenite::Connector::NativeTls(tls_connector);
        match connect_async_tls_with_config(&backend_url, None, false, Some(connector)).await {
            Ok((socket, _)) => socket,
            Err(e) => {
                error!("Failed to connect to backend API WebSocket: {e}");
                return;
            }
        }
    } else {
        match tokio_tungstenite::connect_async(&backend_url).await {
            Ok((socket, _)) => socket,
            Err(e) => {
                error!("Failed to connect to backend API WebSocket: {e}");
                return;
            }
        }
    };

    info!("Connected to backend API WebSocket");

    let (mut client_tx, mut client_rx) = client_socket.split();
    let (mut backend_tx, mut backend_rx) = backend_socket.split();

    loop {
        tokio::select! {
            msg = client_rx.next() => {
                match msg {
                    Some(Ok(AxumMessage::Text(text))) => {
                        if backend_tx.send(TungsteniteMessage::Text(text)).await.is_err() { break; }
                    }
                    Some(Ok(AxumMessage::Binary(data))) => {
                        if backend_tx.send(TungsteniteMessage::Binary(data)).await.is_err() { break; }
                    }
                    Some(Ok(AxumMessage::Ping(data))) => {
                        let _ = client_tx.send(AxumMessage::Pong(data)).await;
                    }
                    Some(Ok(AxumMessage::Pong(_))) => {}
                    Some(Ok(AxumMessage::Close(_))) | None => break,
                    Some(Err(_)) => break,
                }
            }
            msg = backend_rx.next() => {
                match msg {
                    Some(Ok(TungsteniteMessage::Text(text))) => {
                        if client_tx.send(AxumMessage::Text(text)).await.is_err() { break; }
                    }
                    Some(Ok(TungsteniteMessage::Binary(data))) => {
                        if client_tx.send(AxumMessage::Binary(data)).await.is_err() { break; }
                    }
                    Some(Ok(TungsteniteMessage::Ping(data))) => {
                        let _ = backend_tx.send(TungsteniteMessage::Pong(data)).await;
                    }
                    Some(Ok(TungsteniteMessage::Pong(_))) => {}
                    Some(Ok(TungsteniteMessage::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
    debug!("[API_WS_PROXY:{path}] Proxy connection closed");
}

pub async fn ws_proxy(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    Query(params): Query<WsQuery>,
) -> impl IntoResponse {
    let path_parts: Vec<&str> = uri.path().split('/').collect();
    let bot_name = params
        .bot_name
        .filter(|name| name != "ws" && !name.is_empty())
        .or_else(|| {
            path_parts
                .iter()
                .find(|part| {
                    !part.is_empty() && **part != "chat" && **part != "app" && **part != "ws" && **part != "cloud"
                })
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "default".to_string());

    let params_with_bot = WsQuery {
        bot_name: Some(bot_name),
        ..params
    };

    ws.on_upgrade(move |socket| handle_ws_proxy(socket, state, params_with_bot))
}

async fn handle_ws_proxy(
    client_socket: WebSocket,
    state: AppState,
    params: WsQuery,
) {
    let bot_name = params.bot_name.unwrap_or_else(|| "default".to_string());
    let backend_url = format!(
        "{}/ws?bot_name={}&session_id={}&user_id={}",
        state
            .client
            .base_url()
            .replace("https://", "wss://")
            .replace("http://", "ws://"),
        bot_name,
        params.session_id,
        params.user_id
    );

    info!("Proxying WebSocket to: {backend_url}");

    let backend_socket = if backend_url.starts_with("wss://") {
        let mut tls_builder = native_tls::TlsConnector::builder();
        let ca_path = botlib::security::ca_cert_path();
        if let Ok(pem) = std::fs::read(&ca_path) {
            match native_tls::Certificate::from_pem(&pem) {
                Ok(cert) => {
                    tls_builder.add_root_certificate(cert);
                }
                Err(e) => warn!("Invalid platform CA PEM at {ca_path}: {e}"),
            }
        }
        let Ok(tls_connector) = tls_builder.build()
        else {
            error!("Failed to build TLS connector for WebSocket proxy");
            return;
        };
        let connector = tokio_tungstenite::Connector::NativeTls(tls_connector);
        match connect_async_tls_with_config(&backend_url, None, false, Some(connector)).await {
            Ok((socket, _)) => socket,
            Err(e) => {
                error!("Failed to connect to backend WebSocket: {e}");
                return;
            }
        }
    } else {
        match tokio_tungstenite::connect_async(&backend_url).await {
            Ok((socket, _)) => socket,
            Err(e) => {
                error!("Failed to connect to backend WebSocket: {e}");
                return;
            }
        }
    };

    info!("Connected to backend WebSocket");

    let (mut client_tx, mut client_rx) = client_socket.split();
    let (mut backend_tx, mut backend_rx) = backend_socket.split();

    // Proxy loop: respond to pings locally to keep both sides alive
    loop {
        tokio::select! {
            msg = client_rx.next() => {
                match msg {
                    Some(Ok(AxumMessage::Text(text))) => {
                        if backend_tx.send(TungsteniteMessage::Text(text)).await.is_err() { break; }
                    }
                    Some(Ok(AxumMessage::Binary(data))) => {
                        if backend_tx.send(TungsteniteMessage::Binary(data)).await.is_err() { break; }
                    }
                    Some(Ok(AxumMessage::Ping(data))) => {
                        // Respond to client pings directly (backend may be busy)
                        let _ = client_tx.send(AxumMessage::Pong(data)).await;
                    }
                    Some(Ok(AxumMessage::Pong(_))) => {}
                    Some(Ok(AxumMessage::Close(_))) | None => break,
                    Some(Err(_)) => break,
                }
            }
            msg = backend_rx.next() => {
                match msg {
                    Some(Ok(TungsteniteMessage::Text(text))) => {
                        if client_tx.send(AxumMessage::Text(text)).await.is_err() { break; }
                    }
                    Some(Ok(TungsteniteMessage::Binary(data))) => {
                        if client_tx.send(AxumMessage::Binary(data)).await.is_err() { break; }
                    }
                    Some(Ok(TungsteniteMessage::Ping(data))) => {
                        // Respond to backend pings directly (client may be slow)
                        let _ = backend_tx.send(TungsteniteMessage::Pong(data)).await;
                    }
                    Some(Ok(TungsteniteMessage::Pong(_))) => {}
                    Some(Ok(TungsteniteMessage::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
    info!("[WS_PROXY:{bot_name}] Proxy connection closed");
}

pub async fn ws_task_progress_proxy(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<OptionalWsQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_task_progress_ws_proxy(socket, state, params))
}

async fn handle_task_progress_ws_proxy(
    client_socket: WebSocket,
    state: AppState,
    params: OptionalWsQuery,
) {
    let mut backend_url = format!(
        "{}/ws/task-progress",
        state
            .client
            .base_url()
            .replace("https://", "wss://")
            .replace("http://", "ws://"),
    );

    if let Some(task_id) = &params.task_id {
        backend_url = format!("{}/{}", backend_url, task_id);
    }

    info!("Proxying task-progress WebSocket to: {backend_url}");

    let backend_socket = if backend_url.starts_with("wss://") {
        let mut tls_builder = native_tls::TlsConnector::builder();
        let ca_path = botlib::security::ca_cert_path();
        if let Ok(pem) = std::fs::read(&ca_path) {
            match native_tls::Certificate::from_pem(&pem) {
                Ok(cert) => {
                    tls_builder.add_root_certificate(cert);
                }
                Err(e) => warn!("Invalid platform CA PEM at {ca_path}: {e}"),
            }
        }
        let Ok(tls_connector) = tls_builder.build()
        else {
            error!("Failed to build TLS connector for task-progress");
            return;
        };
        let connector = tokio_tungstenite::Connector::NativeTls(tls_connector);
        match connect_async_tls_with_config(&backend_url, None, false, Some(connector)).await {
            Ok((socket, _)) => socket,
            Err(e) => {
                error!("Failed to connect to backend task-progress WebSocket: {e}");
                return;
            }
        }
    } else {
        match tokio_tungstenite::connect_async(&backend_url).await {
            Ok((socket, _)) => socket,
            Err(e) => {
                error!("Failed to connect to backend task-progress WebSocket: {e}");
                return;
            }
        }
    };

    info!("Connected to backend task-progress WebSocket");

    let (mut client_tx, mut client_rx) = client_socket.split();
    let (mut backend_tx, mut backend_rx) = backend_socket.split();

    let client_to_backend = async {
        while let Some(msg) = client_rx.next().await {
            match msg {
                Ok(AxumMessage::Text(text)) => {
                    let res: Result<(), tungstenite::Error> =
                        backend_tx.send(TungsteniteMessage::Text(text)).await;
                    if res.is_err() {
                        break;
                    }
                }
                Ok(AxumMessage::Binary(data)) => {
                    let res: Result<(), tungstenite::Error> =
                        backend_tx.send(TungsteniteMessage::Binary(data)).await;
                    if res.is_err() {
                        break;
                    }
                }
                Ok(AxumMessage::Ping(data)) => {
                    let res: Result<(), tungstenite::Error> =
                        backend_tx.send(TungsteniteMessage::Ping(data)).await;
                    if res.is_err() {
                        break;
                    }
                }
                Ok(AxumMessage::Pong(data)) => {
                    let res: Result<(), tungstenite::Error> =
                        backend_tx.send(TungsteniteMessage::Pong(data)).await;
                    if res.is_err() {
                        break;
                    }
                }
                Ok(AxumMessage::Close(_)) | Err(_) => break,
            }
        }
    };

    let backend_to_client = async {
        while let Some(msg) =
            backend_rx.next().await as Option<Result<TungsteniteMessage, tungstenite::Error>>
        {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    let is_manifest = text.contains("manifest_update");
                    if is_manifest {
                    } else if text.contains("task_progress") {
                        debug!("[WS_PROXY] Forwarding task_progress to client");
                    }
                    match client_tx.send(AxumMessage::Text(text)).await {
                        Ok(()) => {
                            if is_manifest {
                            }
                        }
                        Err(e) => {
                            error!("[WS_PROXY] Failed to send message to client: {:?}", e);
                            break;
                        }
                    }
                }
                Ok(TungsteniteMessage::Binary(data)) => {
                    if client_tx.send(AxumMessage::Binary(data)).await.is_err() {
                        break;
                    }
                }
                Ok(TungsteniteMessage::Ping(data)) => {
                    if client_tx.send(AxumMessage::Ping(data)).await.is_err() {
                        break;
                    }
                }
                Ok(TungsteniteMessage::Pong(data)) => {
                    if client_tx.send(AxumMessage::Pong(data)).await.is_err() {
                        break;
                    }
                }
                Ok(TungsteniteMessage::Close(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }
    };

    tokio::select! {
        () = client_to_backend => info!("Task-progress client connection closed"),
        () = backend_to_client => info!("Task-progress backend connection closed"),
    }
}
