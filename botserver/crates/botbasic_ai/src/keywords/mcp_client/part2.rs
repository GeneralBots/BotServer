use super::*;

impl Default for McpConnection {
    fn default() -> Self {
        Self {
            connection_type: ConnectionType::Http,
            url: "".to_string(),
            port: None,
            timeout_seconds: 30,
            max_retries: 3,
            retry_backoff_ms: 1000,
            keep_alive: true,
            tls_config: None,
        }
    }
}
