use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::ws::{Message as AxumMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::models::DesktopSession;
use crate::session_manager::SessionManager;
use crate::types::{mask_ip, ConnectionMeta, WsProxyMessage};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum lifetime for a proxied connection (4 hours).
const MAX_LIFETIME: Duration = Duration::from_secs(4 * 60 * 60);

/// Idle timeout — no data flowing for 30 minutes triggers disconnect.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Maximum frame size accepted over WebSocket (1 MiB).
const MAX_FRAME_SIZE: usize = 1024 * 1024;

/// Buffer size for the relay channels.
const RELAY_CHANNEL_SIZE: usize = 4096;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("TCP connect failed: {0}")]
    TcpConnect(std::io::Error),

    #[error("WebSocket upgrade failed: {0}")]
    WsUpgrade(String),

    #[error("session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("connection timed out")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Handle an incoming WebSocket connection by proxying it to the target TCP
/// host:port. This is the main entry called from the axum handler.
pub async fn handle_ws_connection(ws: WebSocket, session: DesktopSession, session_manager: SessionManager) {
    let session_id = session.id;
    let target = format!("{}:{}", session.target_host, session.target_port);
    let client_ip = mask_ip(&session.client_ip);

    info!(
        "WS proxy starting: session={} target={} client={}",
        session_id, target, client_ip,
    );

    // -- TCP connect --------------------------------------------------------

    let tcp_stream = match TcpStream::connect(&target).await {
        Ok(s) => s,
        Err(e) => {
            error!("TCP connect to {} failed: {}", target, e);
            let _ = session_manager.remove_session(session_id).await;
            return;
        }
    };

    let _ = tcp_stream.set_nodelay(true);
    info!("TCP connected to {} for session {}", target, session_id);

    // Update session status
    session_manager.mark_connected(session_id).await;

    // -- Send connected metadata -------------------------------------------

    let meta = WsProxyMessage::Connected(ConnectionMeta {
        connection_id: session_id,
        target_host: session.target_host.clone(),
        target_port: session.target_port,
        connected_at: chrono::Utc::now(),
    });
    let meta_json = serde_json::to_string(&meta).unwrap_or_default();
    // We'll send after split below

    // -- Split streams for bidirectional relay -----------------------------

    let (mut ws_sink, mut ws_source) = ws.split();
    let (mut tcp_reader, mut tcp_writer) = tcp_stream.into_split();

    // Send connected metadata
    if ws_sink.send(AxumMessage::Text(meta_json)).await.is_err() {
        warn!("Failed to send connected metadata to client");
    }

    let (ws_to_tcp_tx, mut ws_to_tcp_rx) = mpsc::channel::<Vec<u8>>(RELAY_CHANNEL_SIZE);
    let (tcp_to_ws_tx, mut tcp_to_ws_rx) = mpsc::channel::<Vec<u8>>(RELAY_CHANNEL_SIZE);

    // --- Task: WebSocket → TCP -------------------------------------------
    let ws_to_tcp_handle = tokio::spawn(async move {
        while let Some(msg_result) = ws_source.next().await {
            match msg_result {
                Ok(AxumMessage::Binary(data)) => {
                    if data.len() > MAX_FRAME_SIZE {
                        warn!("Binary frame too large ({} bytes), dropping", data.len());
                        continue;
                    }
                    if ws_to_tcp_tx.send(data.to_vec()).await.is_err() {
                        break;
                    }
                }
                Ok(AxumMessage::Text(text)) => {
                    let text_bytes = text.as_bytes().to_vec();
                    if text_bytes.len() > MAX_FRAME_SIZE {
                        warn!("Text frame too large ({} bytes), dropping", text_bytes.len());
                        continue;
                    }
                    if ws_to_tcp_tx.send(text_bytes).await.is_err() {
                        break;
                    }
                }
                Ok(AxumMessage::Close(_)) => {
                    debug!("WebSocket close received");
                    break;
                }
                Ok(AxumMessage::Ping(data)) => {
                    debug!("WS ping: {} bytes", data.len());
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("WebSocket read error: {}", e);
                    break;
                }
            }
        }
        debug!("WS→TCP relay ended");
    });

    // --- Task: TCP → WebSocket -------------------------------------------
    let tcp_to_ws_handle = tokio::spawn(async move {
        let mut buf = vec![0u8; 8192];
        loop {
            match tcp_reader.read(&mut buf).await {
                Ok(0) => {
                    debug!("TCP EOF");
                    break;
                }
                Ok(n) => {
                    if tcp_to_ws_tx.send(buf[..n].to_vec()).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!("TCP read error: {}", e);
                    break;
                }
            }
        }
        debug!("TCP→WS relay ended");
    });

    // --- Task: forward WS → TCP data through TCP writer -------------------
    let session_id_ws = session_id;
    let sm_ws = session_manager.clone();
    let ws_to_tcp_forward = tokio::spawn(async move {
        while let Some(data) = ws_to_tcp_rx.recv().await {
            if tcp_writer.write_all(&data).await.is_err() {
                break;
            }
            sm_ws.add_bytes(session_id_ws, data.len() as i64, 0).await;
        }
    });

    // --- Task: forward TCP → WS data through WS sink ---------------------
    let session_id_tcp = session_id;
    let sm_tcp = session_manager.clone();
    let tcp_to_ws_forward = tokio::spawn(async move {
        while let Some(data) = tcp_to_ws_rx.recv().await {
            let msg = AxumMessage::Binary(data.clone());
            if ws_sink.send(msg).await.is_err() {
                break;
            }
            sm_tcp
                .add_bytes(session_id_tcp, 0, data.len() as i64)
                .await;
        }
    });

    // -- Idle timeout & max lifetime monitor --------------------------------

    let sm_monitor = session_manager.clone();
    let monitor_handle = tokio::spawn(async move {
        let start = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            if start.elapsed() >= MAX_LIFETIME {
                warn!("Session {} hit max lifetime ({:?})", session_id, MAX_LIFETIME);
                let _ = sm_monitor.disconnect_session(session_id).await;
                break;
            }

            let is_idle = sm_monitor.is_session_idle(session_id).await;

            if is_idle {
                warn!("Session {} idle timeout ({:?})", session_id, IDLE_TIMEOUT);
                let _ = sm_monitor.disconnect_session(session_id).await;
                break;
            }
        }
    });

    // -- Wait for any task to finish (signals end of connection) -------------

    tokio::select! {
        _ = ws_to_tcp_handle => {},
        _ = tcp_to_ws_handle => {},
        _ = ws_to_tcp_forward => {},
        _ = tcp_to_ws_forward => {},
        _ = monitor_handle => {},
    }

    // -- Cleanup ------------------------------------------------------------

    info!("WS proxy closing: session={}", session_id);
    let _ = session_manager.disconnect_session(session_id).await;
}

// ---------------------------------------------------------------------------
// Helper: extract client IP from ConnectInfo
// ---------------------------------------------------------------------------

pub fn extract_client_ip(addr: &SocketAddr) -> String {
    match addr {
        SocketAddr::V4(v4) => v4.ip().to_string(),
        SocketAddr::V6(v6) => match v6.ip().to_ipv4_mapped() {
            Some(v4) => v4.to_string(),
            None => v6.ip().to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Health check helper
// ---------------------------------------------------------------------------

/// Probe whether a TCP port is reachable within a timeout.
pub async fn tcp_health_check(host: &str, port: u16) -> (bool, Option<u64>, Option<String>) {
    let target = format!("{host}:{port}");
    let start = tokio::time::Instant::now();

    match tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&target)).await {
        Ok(Ok(_)) => {
            let latency = start.elapsed().as_millis() as u64;
            (true, Some(latency), None)
        }
        Ok(Err(e)) => (false, None, Some(e.to_string())),
        Err(_) => (false, None, Some("connection timed out".into())),
    }
}

/// Probe an RDP endpoint: connect and confirm the peer speaks RDP by reading
/// the first TPKT header byte (0x03 = X.224 data TPDU), which every RDP
/// server emits at the start of the negotiation.
///
/// Unlike a plain TCP probe this verifies the port actually serves RDP and
/// not, say, a database that happens to listen there.
pub async fn rdp_health_check(host: &str, port: u16) -> (bool, Option<u64>, Option<String>) {
    let target = format!("{host}:{port}");
    let start = tokio::time::Instant::now();

    let stream = match tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&target)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return (false, None, Some(e.to_string())),
        Err(_) => return (false, None, Some("connection timed out".into())),
    };

    let (mut reader, _writer) = stream.into_split();
    let mut header = [0u8; 4];
    let read = tokio::time::timeout(Duration::from_secs(5), reader.read_exact(&mut header)).await;

    let latency = start.elapsed().as_millis() as u64;
    match read {
        Ok(Ok(_)) if header[0] == 0x03 => (true, Some(latency), None),
        Ok(Ok(_)) => (
            false,
            Some(latency),
            Some(format!("peer is not an RDP server (first byte 0x{:02x})", header[0])),
        ),
        Ok(Err(e)) => (false, Some(latency), Some(format!("RDP handshake read failed: {e}"))),
        Err(_) => (false, Some(latency), Some("RDP handshake timed out".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_client_ip_v4() {
        let addr: SocketAddr = "192.168.1.100:12345".parse().unwrap();
        assert_eq!(extract_client_ip(&addr), "192.168.1.100");
    }

    #[test]
    fn test_max_frame_size_constant() {
        assert_eq!(MAX_FRAME_SIZE, 1024 * 1024);
    }

    #[test]
    fn test_relay_channel_size() {
        assert!(RELAY_CHANNEL_SIZE > 0);
    }

    #[tokio::test]
    async fn test_rdp_health_check_rejects_non_rdp_peer() {
        // A TCP listener that is not an RDP server must be reported as such
        // rather than merely reachable.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            // Speak like a plain TCP service: accept and stay silent.
            if let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(b"NO").await;
            }
        });

        let (ok, _latency, err) = rdp_health_check(&addr.ip().to_string(), addr.port()).await;
        assert!(!ok);
        assert!(err.is_some());
    }
}
