use super::*;

impl Default for GatewayState {
    fn default() -> Self {
        let keys = std::env::var("DEPLOY_KEYS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            deploy_keys: keys,
            incus_socket: std::env::var("INCUS_SOCKET")
                .unwrap_or_else(|_| "/var/lib/incus/unix.socket".to_string()),
            caddy_api_url: {
                let (caddy, _vibe) = botcoresecrets::app_runtime();
                if caddy.is_empty() {
                    std::env::var("CADDY_API_URL").unwrap_or_else(|_| "http://127.0.0.1:2019".to_string())
                } else {
                    caddy
                }
            },
            sites_root: std::env::var("SITES_ROOT")
                .unwrap_or_else(|_| "/var/www".to_string()),
        }
    }
}
