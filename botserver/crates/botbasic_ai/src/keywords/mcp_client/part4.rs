use super::*;

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_timeout_seconds: 30,
            max_concurrent_requests: 10,
            cache_enabled: true,
            cache_ttl_seconds: 300,
            audit_enabled: true,
            health_check_interval_seconds: 60,
            auto_retry: true,
            circuit_breaker_threshold: 5,
        }
    }
}
impl std::fmt::Debug for McpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient")
            .field("config", &self.config)
            .field("servers_count", &self.servers.len())
            .finish_non_exhaustive()
    }
}
